package main

import (
	"bytes"
	"errors"
	"fmt"
	"io"
	"mime/multipart"
	"net/http"
	"os"
	"path/filepath"
	"strconv"
	"time"

	"encoding/json"

	"gopkg.in/guregu/null.v4"
)

type Login struct {
	Email    string `json:"email"`
	Password string `json:"password"`
}

type LoginResponse struct {
	Token string `json:"token"`
}

func Authorize(login string, password string) (string, error) {
	authBody := Login{Email: login, Password: password}

	reqBody, err := json.Marshal(authBody)

	resp := sendPostRequest("https://dev.testwise.pro/api/v1/cli/auth", &reqBody)
	if err != nil {
		fmt.Println("Error while creating auth json: ", err.Error())
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return "", errors.New("Can't authorize")
	}
	bodyBytes, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", err
	}
	var respData LoginResponse
	err = json.Unmarshal(bodyBytes, &respData)

	return respData.Token, nil
}

func sendPostRequest(url string, reqBody *[]byte) *http.Response {

	bodyReader := bytes.NewReader(*reqBody)

	req, err := http.NewRequest(http.MethodPost, url, bodyReader)
	if err != nil {
		fmt.Println("Error :", err.Error())
		return nil
	}
	req.Header.Set("Content-Type", "devlication/json")

	client := &http.Client{}
	res, err := client.Do(req)
	if err != nil {
		fmt.Println("Error :", err.Error())
		return nil
	}
	return res
}

func sendGetRequest(url string, token string) *http.Response {
	req, err := http.NewRequest(http.MethodGet, url, nil)
	req.Header.Set("Authorization", "Bearer "+token)
	client := &http.Client{}
	res, err := client.Do(req)
	if err != nil {
		fmt.Println("Error :", err.Error())
		return nil
	}
	return res

}

//{"run_id":"0dfe9125-dad5-42c9-b642-5599530caa79","status":"ok"}

type CreateRunResponse struct {
	RunID  string `json:"run_id"`
	Status string `json:"status"`
}

func SendNewRunWithKey(apiKey string, apkPath string, testApkPath string, commitName string, commitLink string) (string, error) {
	apkFile, err := os.Open(apkPath)
	if err != nil {
		fmt.Println("Can't read apk file")
		return "", err
	}
	defer apkFile.Close()
	testApkFile, err := os.Open(testApkPath)
	if err != nil {
		fmt.Println("Can't read testapk file")
		return "", err
	}
	defer testApkFile.Close()

	body := &bytes.Buffer{}
	writer := multipart.NewWriter(body)
	part, _ := writer.CreateFormFile("app", filepath.Base(apkFile.Name()))
	io.Copy(part, apkFile)
	part2, _ := writer.CreateFormFile("testapp", filepath.Base(apkFile.Name()))

	io.Copy(part2, testApkFile)
	if len(commitName) > 0 {
		writer.WriteField("name", commitName)
	}
	if len(commitLink) > 0 {
		writer.WriteField("link", commitLink)
	}

	writer.Close()

	r, err := http.NewRequest("POST", "https://dev.testwise.pro/api/v1/run?api_key=" + apiKey, body)
  if err != nil {
    fmt.Println(err)
  }
	r.Header.Add("Content-Type", writer.FormDataContentType())
	client := &http.Client{}
	resp, _ := client.Do(r)
	bodyBytes, err := io.ReadAll(resp.Body)
	if err != nil {
    fmt.Println(err)
		return "", err
	}
  fmt.Println(string(bodyBytes))
	var respData CreateRunResponse
	err = json.Unmarshal(bodyBytes, &respData)

	return respData.RunID, nil
}


// deprecate in October 2023
func SendNewRun(token string, apkPath string, testApkPath string, commitName string, commitLink string) (string, error) {
	apkFile, err := os.Open(apkPath)
	if err != nil {
		fmt.Println("Can't read apk file")
		return "", err
	}
	defer apkFile.Close()
	testApkFile, err := os.Open(testApkPath)
	if err != nil {
		fmt.Println("Can't read testapk file")
		return "", err
	}
	defer testApkFile.Close()

	body := &bytes.Buffer{}
	writer := multipart.NewWriter(body)
	part, _ := writer.CreateFormFile("app", filepath.Base(apkFile.Name()))
	io.Copy(part, apkFile)
	part2, _ := writer.CreateFormFile("testapp", filepath.Base(apkFile.Name()))

	io.Copy(part2, testApkFile)
	if len(commitName) > 0 {
		writer.WriteField("name", commitName)
	}
	if len(commitLink) > 0 {
		writer.WriteField("link", commitLink)
	}

	writer.Close()

	r, _ := http.NewRequest("POST", "https://dev.testwise.pro/api/v1/run", body)
	r.Header.Add("Content-Type", writer.FormDataContentType())
	r.Header.Add("Authorization", "Bearer "+token)
	client := &http.Client{}
	resp, _ := client.Do(r)
	bodyBytes, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", err
	}
	var respData CreateRunResponse
	err = json.Unmarshal(bodyBytes, &respData)

	return respData.RunID, nil
}

type RunStats struct {
	ID           string      `json:"id"`
	Name         null.String `json:"name"`
	Link         null.String `json:"link"`
	State        string      `json:"state"`
	Completed    null.Time   `json:"completed,omitempty"`
	Ignored      null.Int    `json:"ignored"`
	Passed       null.Int    `json:"passed"`
	Failed       null.Int    `json:"failed"`
	TotalRunTime float32     `json:"total_run_time"`
	TestDoneAt   null.Time   `json:"tests_done"`
	CreatedAt    time.Time   `json:"created"`
	UpdatedAt    time.Time   `json:"updated"`
}


// Deprecate after October 2023
func WaitRunForEnd(runId string, token string) (string, error) {
	var respData RunStats
	for {
		client := &http.Client{}
		req, err := http.NewRequest("GET", "https://dev.testwise.pro/api/v1/run/"+runId, nil)
		if err != nil {
			return "", err
		}
		req.Header.Add("Authorization", "Bearer "+token)
		resp, err := client.Do(req)
		if err != nil {
			return "", err
		}
		bodyBytes, err := io.ReadAll(resp.Body)
		if err != nil {
			return "", err
		}

		err = json.Unmarshal(bodyBytes, &respData)
		if respData.Completed.Valid == true {
			break
		}
		time.Sleep(5 * time.Second)
	}
	fmt.Println("Allure report - https://dev.testwise.pro/api/v1/report/" + respData.ID)
	fmt.Println("Passed - " + strconv.Itoa(int(respData.Passed.Int64)))
	fmt.Println("Failed - " + strconv.Itoa(int(respData.Failed.Int64)))
	fmt.Println("Ignored - " + strconv.Itoa(int(respData.Ignored.Int64)))
	return respData.State, nil
}

func WaitRunForEndWithApiKey(runId string, apiKey string) (string, error) {
	var respData RunStats
	for {
		client := &http.Client{}
		req, err := http.NewRequest("GET", "https://dev.testwise.pro/api/v1/run/"+runId + "?api_key=" + apiKey, nil)
		if err != nil {
			return "", err
		}
		resp, err := client.Do(req)
		if err != nil {
			return "", err
		}
		bodyBytes, err := io.ReadAll(resp.Body)
		if err != nil {
			return "", err
		}

		err = json.Unmarshal(bodyBytes, &respData)
		if respData.Completed.Valid == true {
			break
		}
		time.Sleep(5 * time.Second)
	}
	fmt.Println("Passed - " + strconv.Itoa(int(respData.Passed.Int64)))
	fmt.Println("Failed - " + strconv.Itoa(int(respData.Failed.Int64)))
	fmt.Println("Ignored - " + strconv.Itoa(int(respData.Ignored.Int64)))
	return respData.State, nil
}

type TokenResponse struct {
  Token string `json:"token"`
}

func RequestJwtToken(apiKey string) (string, error) {
    var tokenObj TokenResponse
  		client := &http.Client{}
		req, err := http.NewRequest("GET", "https://dev.testwise.pro/api/v1/user/jwt?api_key=" + apiKey, nil)
		if err != nil {
			return "", err
		}
		resp, err := client.Do(req)
		if err != nil {
			return "", err
		}
		bodyBytes, err := io.ReadAll(resp.Body)
		if err != nil {
			return "", err
		}

		err = json.Unmarshal(bodyBytes, &tokenObj)
    if err != nil {
      return "", err
    }
    return tokenObj.Token, nil

}

