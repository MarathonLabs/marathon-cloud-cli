package allure

import (
	"cli/request"
	"encoding/json"
	"fmt"
	"github.com/otiai10/copy"
	"io"
	"io/ioutil"
	"os"
	"path"
	"path/filepath"
	"strings"
	"sync"
	"time"
)

type ArtifactTree struct {
	ID     string `json:"id"`
	IsFile bool   `json:"is_file"`
	Name   string `json:"name"`
}

type FileNode struct {
	ID       string `json:"id"`
	IsFile   bool   `json:"is_file"`
	Name     string `json:"name"`
	Downloaded bool `json:"downloaded"`
}

var maxConcurrentDownloads = 10 // Limit the number of concurrent downloads.

func GetArtifacts(host string, token string, runId string, whereToSave string) {
  fmt.Println("Start downloading artifacts")

  var fileTree []FileNode
  var wg sync.WaitGroup

  // Semaphore and error channel
  sem := make(chan struct{}, maxConcurrentDownloads)
  errors := make(chan error)

  // Error handling goroutine
  go func() {
      for err := range errors {
          if err != nil {
              fmt.Println("Error during download:", err)
          }
      }
  }()

  // Step 1: Traverse and store file tree
  traverseAndStoreFileTree(host, token, runId, &fileTree, &wg, sem)
  wg.Wait()  // Wait for file tree traversal to complete

  // Steps 2-3: Check for new files and redownload failed ones
  for {
    if !downloadFilesAndCheckForNew(host, token, runId, &fileTree, whereToSave, sem, errors, 3) {  // Assuming 3 retries
      break
    }
    time.Sleep(time.Duration(60) * time.Second)
  }

  close(errors)  // Close the error channel after all operations are done

	// Post-processing: move contents from runId folder to root
	relocateContents(whereToSave, runId)
	// Update Allure json paths
	updateJsonPaths(whereToSave)

	fmt.Println("Finish downloading artifacts ")
}

func traverseAndStoreFileTree(host string, token string, folderID string, fileTree *[]FileNode, wg *sync.WaitGroup, sem chan struct{}) {
  sem <- struct{}{} // Acquire semaphore
	resp := request.SendGetRequest("https://"+host+"/api/v1/artifact/"+folderID, token)
  <-sem // Release semaphore

	if resp == nil || resp.Body == nil {
		return
	}
	defer resp.Body.Close()

	bodyBytes, err := io.ReadAll(resp.Body)
	if err != nil {
		fmt.Println("Error reading response:", err.Error())
		return
	}

	var folders []ArtifactTree
	err = json.Unmarshal(bodyBytes, &folders)
	if err != nil {
		fmt.Println("Failed to unmarshal response:", err.Error())
		return
	}

	for _, folder := range folders {
		node := FileNode{
			ID:         folder.ID,
			IsFile:     folder.IsFile,
			Name:       folder.Name,
			Downloaded: false,
		}
		*fileTree = append(*fileTree, node)

		if !folder.IsFile {
			wg.Add(1)
			go func(fID string) {
				defer wg.Done()
				traverseAndStoreFileTree(host, token, fID, fileTree, wg, sem)
			}(folder.ID)
		}
	}
}

func downloadFileWithRetry(host string, token string, fileNode *FileNode, whereToSave string, sem chan struct{}, errors chan<- error, maxRetries int) {
    var err error
    for i := 0; i < maxRetries; i++ {
        sem <- struct{}{} // Acquire semaphore
        err = downloadFile(host, token, fileNode.ID, whereToSave)
        <-sem // Release semaphore

        if err == nil {
            fileNode.Downloaded = true
            return
        }

        // Exponential backoff
        time.Sleep(time.Duration(i) * time.Second)
    }
    errors <- err // Send error to error channel if all retries fail
}

func downloadFilesAndCheckForNew(host string, token string, runId string, fileTree *[]FileNode, whereToSave string, sem chan struct{}, errors chan<- error, maxRetries int) bool {
    newFilesAdded := false
    var notDownloadedCount int
    var wg sync.WaitGroup

    // Retry downloading for files that failed in previous attempts
    for i := range *fileTree {
        if (*fileTree)[i].IsFile && !(*fileTree)[i].Downloaded {
            newFilesAdded = true
            wg.Add(1)
            go func(node *FileNode) {
                defer wg.Done()
                downloadFileWithRetry(host, token, node, whereToSave, sem, errors, maxRetries)
            }(&(*fileTree)[i])
        }
    }
    wg.Wait()

    // Re-traverse the file tree to check for new files
    var newFileTree []FileNode
    traverseAndStoreFileTree(host, token, runId, &newFileTree, &wg, sem)
    wg.Wait() // Wait for re-traversal to complete

    // Check for new files and add them to the fileTree
    for _, newNode := range newFileTree {
        found := false
        for _, existingNode := range *fileTree {
            if newNode.ID == existingNode.ID {
                found = true
                break
            }
        }
        if !found {
            newFilesAdded = true
            notDownloadedCount++
            *fileTree = append(*fileTree, newNode)
        }
    }
  
    fmt.Printf("Number of files not yet downloaded: %d\n", notDownloadedCount)
    return newFilesAdded
}

func downloadFile(host string, token string, fileID string, whereToSave string) error {
	if fileID == "" {
		return fmt.Errorf("empty fileID provided")
	}

	// Split the fileID path to figure out the folder structure and file name.
	keyArray := strings.Split(fileID, "/")
	subFolder := ""
	if len(keyArray) > 1 {
		subFolder = strings.Join(keyArray[:len(keyArray)-1], "/")
	}
	fileName := keyArray[len(keyArray)-1]
	fileFolder := path.Join(whereToSave, subFolder)

	// Ensure the directory structure exists.
	err := os.MkdirAll(fileFolder, os.ModePerm)
	if err != nil {
		return fmt.Errorf("failed to create directory: %v", err)
	}

	// Replace any '#' in the fileID with '%23' for the URL request. This is URL encoding.
	validFileID := strings.ReplaceAll(fileID, "#", "%23")
	resp := request.SendGetRequest("https://"+host+"/api/v1/artifact?key="+validFileID, token)
	defer resp.Body.Close()

	// Create the file at the determined path.
	filePath := path.Join(fileFolder, fileName)
	out, err := os.Create(filePath)
	if err != nil {
		return fmt.Errorf("got error while os.Create: %v", err)
	}
	defer out.Close()

	// Copy the response body (the downloaded data) to our file.
	_, err = io.Copy(out, resp.Body)
	if err != nil {
		return fmt.Errorf("error writing file: %v", err)
	}

	return nil
}

func relocateContents(whereToSave string, runId string) {
	runIdDir := filepath.Join(whereToSave, runId)
	if _, err := os.Stat(runIdDir); os.IsNotExist(err) {
		fmt.Println(runId, "directory does not exist. Skipping relocation.")
		return
	}
	if err := copy.Copy(runIdDir, whereToSave); err != nil {
		fmt.Println("Error copying files:", err)
		return
	}

	// Remove the runId directory
	if err := os.RemoveAll(runIdDir); err != nil {
		fmt.Println("Error removing directory", runIdDir, ":", err)
	}
}

func updateJsonPaths(whereToSave string) {
	// 1. Build the hashmap
	fileMap := make(map[string]string)

	err := filepath.Walk(whereToSave, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}

		if !info.IsDir() {
			filename := filepath.Base(path)
			fileMap[filename] = path
		}
		return nil
	})

	if err != nil {
		fmt.Println("Error walking the path", whereToSave, ":", err)
		return
	}

	// 2. Go through each JSON file and update paths
	allureResultsDir := filepath.Join(whereToSave, "report", "allure-results")
	files, err := ioutil.ReadDir(allureResultsDir)
	if err != nil {
		fmt.Println("Error reading directory", allureResultsDir, ":", err)
		return
	}

	for _, file := range files {
		if filepath.Ext(file.Name()) == ".json" {
			filePath := filepath.Join(allureResultsDir, file.Name())

			data, err := ioutil.ReadFile(filePath)
			if err != nil {
				fmt.Println("Error reading file", filePath, ":", err)
				continue
			}

			var jsonData map[string]interface{}
			if err := json.Unmarshal(data, &jsonData); err != nil {
				fmt.Println("Error unmarshaling JSON data from file", filePath, ":", err)
				continue
			}

			if attachments, ok := jsonData["attachments"].([]interface{}); ok {
				for _, attachment := range attachments {
					if attachMap, ok := attachment.(map[string]interface{}); ok {
						if source, exists := attachMap["source"]; exists {
							if sourceStr, ok := source.(string); ok {
								filename := filepath.Base(sourceStr)

								if newPath, found := fileMap[filename]; found {
									relativePath, err := filepath.Rel(allureResultsDir, newPath)
									if err != nil {
										fmt.Println("Error calculating relative path for", newPath, ":", err)
										continue
									}
									attachMap["source"] = relativePath
								}
							}
						}
					}
				}

				updatedData, err := json.MarshalIndent(jsonData, "", "  ")
				if err != nil {
					fmt.Println("Error marshaling JSON data for file", filePath, ":", err)
					continue
				}

				if err := ioutil.WriteFile(filePath, updatedData, 0644); err != nil {
					fmt.Println("Error writing updated data to file", filePath, ":", err)
				}
			}
		}
	}
}
