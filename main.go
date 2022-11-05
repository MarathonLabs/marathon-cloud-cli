package main

import (
	"fmt"
	"os"
	"time"
)

func main() {
	err := ReadFlags()
	if err != nil {
		fmt.Println("Error reading flags:\n", err.Error())
		os.Exit(7)
	}
	conf := GetConfig()
	login := conf.GetString("LOGIN")
	password := conf.GetString("PASSWORD")
	apk := conf.GetString("APK")
	testApk := conf.GetString("TEST_APK")
	commitName := conf.GetString("NAME")
	commitLink := conf.GetString("LINK")
	token, err := Authorize(login, password)
	if err != nil {
		fmt.Println("Can't login: ", err.Error())
		os.Exit(6)
	}
	fmt.Println(time.Now().Format(time.Stamp), "Creating new run")
	runId, err := SendNewRun(token, apk, testApk, commitName, commitLink)
	if err != nil {
		fmt.Println(err.Error())
		os.Exit(5)
	}
	go Subscribe(token, runId)
	state, err := WaitRunForEnd(runId, token)
	if err != nil {
		fmt.Println(err.Error())
		os.Exit(4)
	}
	if state != "passed" {
		os.Exit(3)
	}
}
