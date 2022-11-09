package main

import (
	"encoding/json"
	"fmt"
	"io"
	"io/ioutil"
	"os"
	"path"
	"strings"
)

type ArtifactTree struct {
	ID     string `json:"id"`
	IsFile bool   `json:"is_file"`
	Name   string `json:"name"`
}

func GetFolder(token string, folder string) *[]ArtifactTree {
	resp := sendGetRequest("https://app.testwise.pro/api/v1/artifact/"+folder, token)

	bodyBytes, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil
	}
	var folders []ArtifactTree
	err = json.Unmarshal(bodyBytes, &folders)
	return &folders

	//

}

func GetFoldersRecursively(token string, folder string, whereToSave string) {
	resp := sendGetRequest("https://app.testwise.pro/api/v1/artifact/"+folder, token)

	bodyBytes, err := io.ReadAll(resp.Body)
	if err != nil {
		return
	}
	var folders []ArtifactTree
	err = json.Unmarshal(bodyBytes, &folders)
	for _, folder := range folders {
		if folder.IsFile == false {
			GetFoldersRecursively(token, folder.ID, whereToSave)
		} else {
			DownloadFile(token, folder.ID, whereToSave)
		}
	}

	//

}

func GetArtifacts(token string, runId string, whereToSave string) {
	rootFolders := GetFolder(token, runId)

	for _, folder := range *rootFolders {
		if strings.HasPrefix(folder.Name, "i-") == true {
			GetFoldersRecursively(token, folder.ID, whereToSave)
		}
	}
}

func DownloadFile(token string, fileID string, whereToSave string) {
	//https://app.testwise.pro/api/v1/artifact?key=32f199b9-9aeb-46d6-812c-fb347a516165/allure-report/04941cb0-6f3d-4dc9-86fe-b8252cd949e6-result.json
	keyArray := strings.Split(fileID, "/")
	subFolder := strings.Join(keyArray[3:len(keyArray)-1], "/")
	fileName := keyArray[len(keyArray)-1]
	extentionArr := strings.Split(fileName, ".")
	extention := extentionArr[len(extentionArr)-1]
	fileFolder := path.Join(whereToSave, subFolder)
	os.MkdirAll(fileFolder, os.ModePerm)
	validFileID := strings.ReplaceAll(fileID, "#", "%23")
	resp := sendGetRequest("https://app.testwise.pro/api/v1/artifact?key="+validFileID, token)
	defer resp.Body.Close()
	filePath := path.Join(fileFolder, fileName)
	out, err := os.Create(filePath)
	if err != nil {
		fmt.Println("Got error while os.Create", err.Error())
		return
	}
	defer out.Close()

	if extention != "json" {
		_, err := io.Copy(out, resp.Body)
		if err != nil {
			fmt.Println("Error writing file", err.Error())
		}
	} else {
		read, err := ioutil.ReadAll(resp.Body)

		newContents := strings.Replace(string(read), "/work/build/reports/marathon/", "../", -1)

		out.Write([]byte(newContents))
		if err != nil {
			panic(err)
		}
	}

}
