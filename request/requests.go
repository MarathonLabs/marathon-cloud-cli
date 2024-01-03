package request

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

func Authorize(host string, login string, password string) (string, error) {
	authBody := Login{Email: login, Password: password}

	reqBody, err := json.Marshal(authBody)

	resp := sendPostRequest("https://"+host+"/api/v1/cli/auth", &reqBody)
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

func SendGetRequest(url string, token string) *http.Response {
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

func SendNewRunWithKey(host string, apiKey string, appPath string, testAppPath string, commitName string, commitLink string, platform string, osVersion string, systemImage string, isolated string, filteringConfigJson string, flavor string) (string, error) {
	body := &bytes.Buffer{}
	writer := multipart.NewWriter(body)
  
	fmt.Println("Application file uploading...")
  appFile, err := os.Open(appPath)
	if err != nil {
		fmt.Println("Can't read apk file")
		return "", err
	}
	defer appFile.Close()
	part, _ := writer.CreateFormFile("app", filepath.Base(appFile.Name()))
	io.Copy(part, appFile)
	fmt.Println("Application file uploading done")

	fmt.Println("Test Application file uploading...")
  testAppFile, err := os.Open(testAppPath)
	if err != nil {
		fmt.Println("Can't read testapk file")
		return "", err
	}
	defer testAppFile.Close()
	part2, _ := writer.CreateFormFile("testapp", filepath.Base(testAppFile.Name()))
	io.Copy(part2, testAppFile)
	fmt.Println("Test Application file uploading done")

	writer.WriteField("platform", platform)
	if len(commitName) > 0 {
		writer.WriteField("name", commitName)
	}
	if len(commitLink) > 0 {
		writer.WriteField("link", commitLink)
	}
	if len(osVersion) > 0 {
		writer.WriteField("osversion", osVersion)
	}
	if isolated == "true" || isolated == "false" {
		writer.WriteField("isolated", isolated)
  }
	if len(systemImage) > 0 {
		writer.WriteField("system_image", systemImage)
	}
  if len(filteringConfigJson) > 0 {
		writer.WriteField("filtering_configuration", filteringConfigJson)
	}
  if len(flavor) > 0 {
		writer.WriteField("flavor", flavor)
	}

	writer.Close()

	r, err := http.NewRequest("POST", "https://"+host+"/api/v1/run?api_key="+apiKey, body)
	if err != nil {
		fmt.Println(err)
	}
	r.Header.Add("Content-Type", writer.FormDataContentType())
	client := &http.Client{}

	fmt.Println("Making request to start the test run...")
	resp, err := client.Do(r)
	if err != nil {
		fmt.Println(err)
		return "", err
	}
  if resp.StatusCode != 200 {
    err = fmt.Errorf("Received error with status code = %d", resp.StatusCode)
		return "", err
  }

	bodyBytes, err := io.ReadAll(resp.Body)
	if err != nil {
		fmt.Println(err)
		return "", err
	}
	var respData CreateRunResponse
	err = json.Unmarshal(bodyBytes, &respData)
  if err != nil {
		fmt.Println(err)
	}

	fmt.Println("The test run was started. RunID=" + respData.RunID)
	return respData.RunID, nil
}

// deprecate in October 2023
func SendNewRun(host string, token string, appPath string, testAppPath string, commitName string, commitLink string, platform string) (string, error) {
	appFile, err := os.Open(appPath)
	if err != nil {
		fmt.Println("Can't read app file")
		return "", err
	}
	defer appFile.Close()
	testAppFile, err := os.Open(testAppPath)
	if err != nil {
		fmt.Println("Can't read testapp file")
		return "", err
	}
	defer testAppFile.Close()

	body := &bytes.Buffer{}
	writer := multipart.NewWriter(body)

	part, _ := writer.CreateFormFile("app", filepath.Base(appFile.Name()))
	io.Copy(part, appFile)

	part2, _ := writer.CreateFormFile("testapp", filepath.Base(testAppFile.Name()))
	io.Copy(part2, testAppFile)

	writer.WriteField("platform", platform)
	if len(commitName) > 0 {
		writer.WriteField("name", commitName)
	}
	if len(commitLink) > 0 {
		writer.WriteField("link", commitLink)
	}

	writer.Close()

	r, _ := http.NewRequest("POST", "https://"+host+"/api/v1/run", body)
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
func WaitRunForEnd(host string, runId string, token string) (string, error) {
	var respData RunStats
	for {
		client := &http.Client{}
		req, err := http.NewRequest("GET", "https://"+host+"/api/v1/run/"+runId, nil)
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
	fmt.Println("Allure report - https://cloud.marathonlabs.io/api/v1/report/" + respData.ID)
	fmt.Println("Passed - " + strconv.Itoa(int(respData.Passed.Int64)))
	fmt.Println("Failed - " + strconv.Itoa(int(respData.Failed.Int64)))
	fmt.Println("Ignored - " + strconv.Itoa(int(respData.Ignored.Int64)))
	return respData.State, nil
}

func WaitRunForEndWithApiKey(host string, runId string, apiKey string) (string, error) {
	fmt.Println("Waiting for the test run finish...")
	var respData RunStats
	for {
		client := &http.Client{}
		req, err := http.NewRequest("GET", "https://"+host+"/api/v1/run/"+runId+"?api_key="+apiKey, nil)
		if err != nil {
			return "", err
		}
		resp, err := client.Do(req)
		if err != nil {
			return "", err
		}
    if resp.StatusCode != 200 {
      fmt.Println(fmt.Sprintf("Status code = %d. Maybe it is a critical error", resp.StatusCode))
      continue
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
	fmt.Println("Allure report - https://cloud.marathonlabs.io/api/v1/report/" + respData.ID)
	fmt.Println("Passed - " + strconv.Itoa(int(respData.Passed.Int64)))
	fmt.Println("Failed - " + strconv.Itoa(int(respData.Failed.Int64)))
	fmt.Println("Ignored - " + strconv.Itoa(int(respData.Ignored.Int64)))
	return respData.State, nil
}

type TokenResponse struct {
	Token string `json:"token"`
}

func RequestJwtToken(host string, apiKey string) (string, error) {
	fmt.Println("Token is requesting...")
	var tokenObj TokenResponse
	client := &http.Client{}
	req, err := http.NewRequest("GET", "https://"+host+"/api/v1/user/jwt?api_key="+apiKey, nil)
	if err != nil {
		return "", err
	}
	resp, err := client.Do(req)
	if err != nil {
		return "", err
	}
  if resp.StatusCode != 200 {
    err = fmt.Errorf("Received error with status code = %d", resp.StatusCode)
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
	fmt.Println("Token was received")
	return tokenObj.Token, nil
}
