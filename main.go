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
  apiKey := conf.GetString("API_KEY")
  apk := conf.GetString("APK")
  testApk := conf.GetString("TEST_APK")
  commitName := conf.GetString("NAME")
  commitLink := conf.GetString("LINK")
  allureOutput := conf.GetString("ALLURE_OUTPUT")
  if len(apiKey) == 0 {
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
    if len(allureOutput) > 0 {
      GetArtifacts(token, runId, allureOutput)
    }
    if err != nil {
      fmt.Println(err.Error())
      os.Exit(4)
    }
    if state != "passed" {
      os.Exit(3)
    }
  } else {
    fmt.Println("Go with api_key")
    jwtToken, err := RequestJwtToken(apiKey) 
    fmt.Println("JWT = ", jwtToken)
    if err != nil {
      fmt.Println(err)
      return
    }
    runId, err := SendNewRunWithKey(apiKey, apk, testApk, commitName, commitLink)
    if err != nil {
      fmt.Println(err.Error())
      os.Exit(5)
    }
    fmt.Println("runId = ", runId)
    go Subscribe(jwtToken, runId)
    state, err := WaitRunForEndWithApiKey(runId, apiKey)
    if len(allureOutput) > 0 {
      GetArtifacts(jwtToken, runId, allureOutput)
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
