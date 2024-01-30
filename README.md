# Marathon Cloud command-line interface

## Installation
For homebrew users:
```bash
brew tap malinskiy/tap
brew install malinskiy/tap/marathon-cloud
```

For docker users:
```bash
docker pull marathonlabs/marathon-cloud:latest
alias marathon-cloud='docker run -v "$(pwd)":/work -it --rm marathonlabs/marathon-cloud:latest'
```

## Usage
```bash
Usage of marathon-cloud:
  -app string
        application filepath. Required
        android example: /home/user/workspace/sample.apk 
        ios example: /home/user/workspace/sample.zip
  -testapp string
        test apk file path. Required
        android example: /home/user/workspace/testSample.apk 
        ios example: /home/user/workspace/sampleUITests-Runner.zip
  -platform string 
        testing platform. Required
        possible values: "Android" or "iOS"
  -api-key string
        api-key for client. Required
  -os-version string
        Android or iOS OS version
  -link string
        link to commit
  -name string
        name for run, for example it could be description of commit
  -o string
        allure raw results output folder
  -system-image string
        OS-specific system image. For Android one of [default,google_apis]. For iOS only [default]
  -isolated bool
        Run each test using isolated execution. Default is false.
  -filter-file string
        File containing test filters in YAML format, following the schema described at https://docs.marathonlabs.io/runner/configuration/filtering/#filtering-logic. 
        For iOS see also https://docs.marathonlabs.io/runner/next/ios#test-plans.
  -flavor string
        Type of tests to run. Default: [native]. Possible values: [native, js-test-appium, python-robotframework-appium].
```
