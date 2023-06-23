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
  -link string
        link to commit
  -name string
        name for run, for example it could be description of commit
  -o string
        allure raw results output folder
```

