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
	"time"
)

type ArtifactTree struct {
	ID     string `json:"id"`
	IsFile bool   `json:"is_file"`
	Name   string `json:"name"`
}

func GetArtifacts(token string, runId string, whereToSave string) {
	rootFolders := getFolder(token, runId)
	if rootFolders == nil || len(*rootFolders) == 0 {
		fmt.Println("Failed to retrieve root folders.")
		return
	}

	for _, folder := range *rootFolders {
		getFoldersRecursively(token, folder.ID, whereToSave)
	}

	// Post-processing: move contents from runId folder to root
	relocateContents(whereToSave, runId)
	// Update Allure json paths
	updateJsonPaths(whereToSave)
}

func getFolder(token string, folder string) *[]ArtifactTree {
	expectedFolders := map[string]bool{
		"bill":        false,
		"devices":     false,
		"html":        false,
		"logs":        false,
		"report":      false,
		"test_result": false,
		"tests":       false,
	}

	var lastRetrievedFolders []ArtifactTree

	for i := 0; i < 3; i++ {
		time.Sleep(10 * time.Second)

		resp := request.SendGetRequest("https://app.testwise.pro/api/v1/artifact/"+folder, token)
		if resp == nil || resp.Body == nil {
			continue
		}

		bodyBytes, _ := io.ReadAll(resp.Body)
		resp.Body.Close() // Always close the body.

		var folders []ArtifactTree
		json.Unmarshal(bodyBytes, &folders)

		// Update the status of found expected folders
		for _, f := range folders {
			if _, exists := expectedFolders[f.Name]; exists {
				expectedFolders[f.Name] = true
			}
		}

		// Check if all expected folders are found
		allFound := true
		for _, found := range expectedFolders {
			if !found {
				allFound = false
				break
			}
		}

		if allFound {
			return &folders
		}

		// Store the last set of folders retrieved
		lastRetrievedFolders = folders
	}

	// If all expected folders aren't found after 3 attempts, return the last set of folders found
	return &lastRetrievedFolders
}

func getFoldersRecursively(token string, folderID string, whereToSave string) {
	resp := request.SendGetRequest("https://app.testwise.pro/api/v1/artifact/"+folderID, token)
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
		if folder.IsFile {
			downloadFile(token, folder.ID, whereToSave)
		} else {
			getFoldersRecursively(token, folder.ID, whereToSave)
		}
	}
}

func downloadFile(token string, fileID string, whereToSave string) {
	if fileID == "" {
		fmt.Println("Empty fileID provided.")
		return
	}

	keyArray := strings.Split(fileID, "/")
	subFolder := ""
	if len(keyArray) > 1 {
		subFolder = strings.Join(keyArray[:len(keyArray)-1], "/")
	}
	fileName := keyArray[len(keyArray)-1]
	fileFolder := path.Join(whereToSave, subFolder)

	err := os.MkdirAll(fileFolder, os.ModePerm)
	if err != nil {
		fmt.Println("Failed to create directory:", err.Error())
		return
	}

	validFileID := strings.ReplaceAll(fileID, "#", "%23")
	resp := request.SendGetRequest("https://app.testwise.pro/api/v1/artifact?key="+validFileID, token)
	defer resp.Body.Close()

	filePath := path.Join(fileFolder, fileName)
	out, err := os.Create(filePath)
	if err != nil {
		fmt.Println("Got error while os.Create:", err.Error())
		return
	}
	defer out.Close()

	_, err = io.Copy(out, resp.Body)
	if err != nil {
		fmt.Println("Error writing file:", err.Error())
	}
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
