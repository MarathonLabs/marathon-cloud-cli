package config

import (
	"errors"
	"flag"
	"os"
	"strings"

	"github.com/spf13/viper"
)

var config *viper.Viper

func ReadFlags() error {
	config = viper.New()

  CONFIG_HOST := flag.String("host", "app.testwise.pro", "Marathon Cloud API host")
	CONFIG_APP := flag.String(
		"app",
		"",
		"application filepath, example: android => /home/user/workspace/sample.apk; iOS => /home/user/workspace/sample.zip. Required")
	CONFIG_TEST_APP := flag.String(
		"testapp",
		"",
		"test app filepath, example: android => /home/user/workspace/testSample.apk; iOS => /home/user/workspace/sampleUITests-Runner.zip. Required")
	CONFIG_COMMIT_NAME := flag.String("name", "", "name for run, for example it could be description of commit")
	CONFIG_COMMIT_LINK := flag.String("link", "", "link to commit")
	CONFIG_ALLURE_OUTPUT := flag.String("o", "", "allure raw results output folder")
	CONFIG_API_KEY := flag.String("api-key", "", "api-key for client. Required")
	CONFIG_LOGIN := flag.String("e", "", "user email, example: user@domain.com. Deprecated")
	CONFIG_PASSWORD := flag.String("p", "", "user password, example: 123456. Deprecated")
	CONFIG_PLATFORM := flag.String("platform", "", "testing platform (Android or iOS only)")
	CONFIG_OS_VERSION := flag.String("os-version", "", "Android or iOS OS version")
	CONFIG_SYSTEM_IMAGE := flag.String("system-image", "", "OS-specific system image. For Android one of [default,google_apis]. For iOS only [default]")

	args := os.Args
	if len(args) > 1 && args[1] == "help" {
		args[1] = "-help"
	}

	flag.Parse()

  config.Set("HOST", *CONFIG_HOST);

	// app
	if len(*CONFIG_APP) > 0 {
		config.Set("APP", *CONFIG_APP)
	} else {
		return errors.New("app filepath must be specified")
	}

	// test app
	if len(*CONFIG_TEST_APP) > 0 {
		config.Set("TEST_APP", *CONFIG_TEST_APP)
	} else {
		return errors.New("testapp filepath must be specified")
	}

	// configPlatformLowerCase
	if *CONFIG_PLATFORM == "" {
		return errors.New("platform must be specified")
	}
	configPlatformLowerCase := strings.ToLower(*CONFIG_PLATFORM)
	var platform string
	if configPlatformLowerCase == "android" {
		platform = "Android"
	} else if configPlatformLowerCase == "ios" {
		platform = "iOS"
	} else {
		return errors.New("platform must be 'Android' or 'iOS'")
	}
	config.Set("PLATFORM", platform)

	// login & password
	if len(*CONFIG_LOGIN) > 0 {
		config.Set("LOGIN", *CONFIG_LOGIN)
	}
	if len(*CONFIG_PASSWORD) > 0 {
		config.Set("PASSWORD", *CONFIG_PASSWORD)
	}

	// api key
	if len(*CONFIG_API_KEY) > 0 {
		config.Set("API_KEY", *CONFIG_API_KEY)
	}

	if len(*CONFIG_API_KEY) == 0 && (len(*CONFIG_LOGIN) == 0 || len(*CONFIG_PASSWORD) == 0) {
		return errors.New("api-key or login with password must be specified")
	}

  config.Set("NAME", *CONFIG_COMMIT_NAME)
	config.Set("LINK", *CONFIG_COMMIT_LINK)
	config.Set("ALLURE_OUTPUT", *CONFIG_ALLURE_OUTPUT)
	config.Set("OS_VERSION", *CONFIG_OS_VERSION)
	config.Set("SYSTEM_IMAGE", *CONFIG_SYSTEM_IMAGE)

	return nil
}

func GetConfig() *viper.Viper {
	return config
}
