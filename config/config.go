package config

import (
	"errors"
	"flag"
	"os"

	"github.com/spf13/viper"
)

var config *viper.Viper

func ReadFlags() error {
	config = viper.New()

	CONFIG_APK := flag.String("apk", "", "application apk filepath, example: /home/user/workspace/app.apk. Required")
	CONFIG_TEST_APK := flag.String("testapk", "", "test apk file path, example: /home/user/workspace/test.apk. Required")
	CONFIG_COMMIT_NAME := flag.String("name", "", "name for run, for example it could be description of commit")
	CONFIG_COMMIT_LINK := flag.String("link", "", "link to commit")
	CONFIG_ALLURE_OUTPUT := flag.String("o", "", "allure raw results output folder")
  CONFIG_ALLURE_API_KEY := flag.String("api-key", "", "api-key for client. Required")
	CONFIG_LOGIN := flag.String("e", "", "user email, example: user@domain.com. Deprecated")
	CONFIG_PASSWORD := flag.String("p", "", "user password, example: 123456. Deprecated")
	args := os.Args
	if len(args) > 1 && args[1] == "help" {
		args[1] = "-help"
	}
	flag.Parse()
	if len(*CONFIG_LOGIN) > 0 {
		config.Set("LOGIN", *CONFIG_LOGIN)
	} 
	if len(*CONFIG_PASSWORD) > 0 {
		config.Set("PASSWORD", *CONFIG_PASSWORD)
	}
	if len(*CONFIG_APK) > 0 {
		config.Set("APK", *CONFIG_APK)
	} else {
		return errors.New("apk filepath must be specified")
	}
	if len(*CONFIG_TEST_APK) > 0 {
		config.Set("TEST_APK", *CONFIG_TEST_APK)
	} else {
		return errors.New("testapk filepath must be specified")
	}
  if len(*CONFIG_ALLURE_API_KEY) > 0 {
    config.Set("API_KEY", *CONFIG_ALLURE_API_KEY)
  }

  if len(*CONFIG_ALLURE_API_KEY) == 0 && (len(*CONFIG_LOGIN) == 0 || len(*CONFIG_PASSWORD) == 0) {
    return errors.New("api-key or login with password must be specified")
  }

	config.Set("NAME", *CONFIG_COMMIT_NAME)
	config.Set("LINK", *CONFIG_COMMIT_LINK)
	config.Set("ALLURE_OUTPUT", *CONFIG_ALLURE_OUTPUT)

	return nil
}

func GetConfig() *viper.Viper {
	return config
}
