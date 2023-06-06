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
  -apk string
        application apk filepath, example: /home/user/workspace/app.apk. Required
  -testapk string
        test apk file path, example: /home/user/workspace/test.apk. Required
  -api-key string
        api-key for client. Required
  -link string
        link to commit
  -name string
        name for run, for example it could be description of commit
  -o string
        allure raw results output folder
```

