package filter

import (
    "encoding/json"
    "errors"
    "io/ioutil"

    "gopkg.in/yaml.v2"
)

type Configuration struct {
    FilteringConfig FilteringConfiguration `json:"filteringConfiguration,omitempty" yaml:"filteringConfiguration"`
}

type FilteringConfiguration struct {
    Allowlist *[]Filter `json:"allowlist,omitempty" yaml:"allowlist"`
    Blocklist *[]Filter `json:"blocklist,omitempty" yaml:"blocklist"`
}

type Filter struct {
    Type    string   `json:"type,omitempty" yaml:"type"`
    Regex   string   `json:"regex,omitempty" yaml:"regex,omitempty"`
    Values  []string `json:"values,omitempty" yaml:"values,omitempty"`
    Filters []Filter `json:"filters,omitempty" yaml:"filters,omitempty"`
    Op      string   `json:"op,omitempty" yaml:"op,omitempty"`
    File    string   `json:"file,omitempty" yaml:"file,omitempty"`
}

func ValidateYAMLAndConvertToJSON(filePath string) (string, error) {
    yamlData, err := ioutil.ReadFile(filePath)
    if err != nil {
        return "", err
    }

    var config Configuration
    err = yaml.Unmarshal(yamlData, &config)
    if err != nil {
        return "", err
    }

    if err := validateFilters(config.FilteringConfig.Allowlist); err != nil {
        return "", err
    }
    if err := validateFilters(config.FilteringConfig.Blocklist); err != nil {
        return "", err
    }

    jsonOutput, err := json.Marshal(config)
    if err != nil {
        return "", err
    }

    return string(jsonOutput), nil
}

func validateFilters(filters *[]Filter) error {
    if filters == nil {
        return nil
    }

    validTypes := map[string]bool{
        "fully-qualified-class-name": true,
        "fully-qualified-test-name":  true,
        "simple-class-name":          true,
        "package":                    true,
        "method":                     true,
        "annotation":                 true,
        "allure":                     true,
        "composition":                true,
    }

    for _, filter := range *filters {
        if filter.Type == "fragmentation" {
            return errors.New("the 'Fragmented execution of tests' feature (type: 'fragmentation') is not supported in the cloud")
        }
        if !validTypes[filter.Type] {
            return errors.New("invalid filter type: " + filter.Type)
        }
        if filter.Type == "composition" {
            if filter.Op == "" || len(filter.Filters) == 0 {
                return errors.New("composition type must have 'op' and 'filters' fields initialized")
            }
            if err := validateFilters(&filter.Filters); err != nil {
                return err
            }
        } else {
            if err := validateNonCompositionFilter(filter); err != nil {
                return err
            }
        }
    }
    return nil
}

func validateNonCompositionFilter(filter Filter) error {
    fieldsInitialized := 0
    if filter.Regex != "" {
        fieldsInitialized++
    }
    if len(filter.Values) > 0 {
        fieldsInitialized++
    }
    if filter.File != "" {
        return errors.New("the 'file' field is not supported. Please include all values directly in the YAML")
    }
    if fieldsInitialized > 1 {
        return errors.New("only one of [regex, values] can be specified for type: " + filter.Type)
    }
    if fieldsInitialized == 0 {
        return errors.New("at least one of [regex, values] should be specified for type: " + filter.Type)
    }
    return nil
}
