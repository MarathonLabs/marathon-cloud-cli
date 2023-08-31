package main

import (
	"cli/allure"
	"cli/config"
	"cli/request"
	"fmt"
	"os"
	"time"
)

func main() {
	err := config.ReadFlags()
	if err != nil {
		fmt.Println("Error reading flags:\n", err.Error())
		os.Exit(7)
	}

	conf := config.GetConfig()
	login := conf.GetString("LOGIN")
	password := conf.GetString("PASSWORD")
	apiKey := conf.GetString("API_KEY")
	app := conf.GetString("APP")
	testApp := conf.GetString("TEST_APP")
	commitName := conf.GetString("NAME")
	commitLink := conf.GetString("LINK")
	allureOutput := conf.GetString("ALLURE_OUTPUT")
	platform := conf.GetString("PLATFORM")
	osVersion := conf.GetString("OS_VERSION")

	if len(apiKey) == 0 {
		token, err := request.Authorize(login, password)
		if err != nil {
			fmt.Println("Can't login: ", err.Error())
			os.Exit(6)
		}
		fmt.Println(time.Now().Format(time.Stamp), "Creating new run")
		runId, err := request.SendNewRun(token, app, testApp, commitName, commitLink, platform)
		if err != nil {
			fmt.Println(err.Error())
			os.Exit(5)
		}
		go request.Subscribe(token, runId)

		state, err := request.WaitRunForEnd(runId, token)
		if len(allureOutput) > 0 {
			allure.GetArtifacts(token, runId, allureOutput)
		}
		if err != nil {
			fmt.Println(err.Error())
			os.Exit(4)
		}
		if state != "passed" {
			os.Exit(3)
		}
	} else {
		jwtToken, err := request.RequestJwtToken(apiKey)
		if err != nil {
			fmt.Println(err)
			return
		}
		runId, err := request.SendNewRunWithKey(apiKey, app, testApp, commitName, commitLink, platform, osVersion)
		if err != nil {
			fmt.Println(err.Error())
			os.Exit(5)
		}
		go request.Subscribe(jwtToken, runId)
		state, err := request.WaitRunForEndWithApiKey(runId, apiKey)
		if len(allureOutput) > 0 {
			allure.GetArtifacts(jwtToken, runId, allureOutput)
		}
		if err != nil {
			fmt.Println(err.Error())
			os.Exit(4)
		}
		if state != "passed" {
			os.Exit(3)
		}
	}
}
