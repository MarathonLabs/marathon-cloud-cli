package filter

import (
    "encoding/json"
    "io/ioutil"
    "reflect"
    "testing"
    "strings"

    "gopkg.in/yaml.v2"
)

func TestValidateYAMLAndConvertToJSONFragmentation(t *testing.T) {
    _, err := ValidateYAMLAndConvertToJSON("./testdata/fragmentation.yaml")
    if err == nil || err.Error() != "the 'Fragmented execution of tests' feature (type: 'fragmentation') is not supported in the cloud" {
        t.Errorf("ValidateYAMLAndConvertToJSON failed to detect fragmentation feature")
    }
}

func TestValidateYAMLAndConvertToJSONFileType(t *testing.T) {
    _, err := ValidateYAMLAndConvertToJSON("./testdata/filetype.yaml")
    if err == nil || err.Error() != "the 'file' field is not supported. Please include all values directly in the YAML" {
        t.Errorf("ValidateYAMLAndConvertToJSON failed to detect file filter parameter")
    }
}

func TestValidateYAMLAndConvertToJSONInvalidYAML(t *testing.T) {
    _, err := ValidateYAMLAndConvertToJSON("./testdata/invalid.yaml")
    if err == nil {
        t.Errorf("ValidateYAMLAndConvertToJSON should have failed for invalid YAML format")
    }
}

func TestValidateYAMLAndConvertToJSONGrammarError(t *testing.T) {
    _, err := ValidateYAMLAndConvertToJSON("./testdata/grammarError.yaml")
    if err == nil {
        t.Errorf("ValidateYAMLAndConvertToJSON should have failed for grammar errors in YAML")
    }
}

func TestValidateYAMLAndConvertToJSONUnknownType(t *testing.T) {
    _, err := ValidateYAMLAndConvertToJSON("./testdata/unknownType.yaml")
    if err == nil || !strings.Contains(err.Error(), "invalid filter type") {
        t.Errorf("ValidateYAMLAndConvertToJSON should have failed for unknown filter type")
    }
}

func TestValidateYAMLAndConvertToJSONCorrectTypeNoFields(t *testing.T) {
    _, err := ValidateYAMLAndConvertToJSON("./testdata/correctTypeNoFields.yaml")
    if err == nil {
        t.Errorf("ValidateYAMLAndConvertToJSON should have failed for correct type with no additional fields")
    }
}

func TestValidateYAMLAndConvertToJSONCorrectTypeTwoFields(t *testing.T) {
    _, err := ValidateYAMLAndConvertToJSON("./testdata/correctTypeTwoFields.yaml")
    if err == nil || !strings.Contains(err.Error(), "only one of [regex, values] can be specified") {
        t.Errorf("ValidateYAMLAndConvertToJSON should have failed for correct type with two fields")
    }
}

func TestValidateYAMLAndConvertToJSONInvalidCompositionFields(t *testing.T) {
    _, err := ValidateYAMLAndConvertToJSON("./testdata/invalidCompositionFields.yaml")
    if err == nil || !strings.Contains(err.Error(), "composition type must have 'op' and 'filters' fields initialized") {
        t.Errorf("ValidateYAMLAndConvertToJSON should have failed for invalid composition fields")
    }
}

func TestValidateYAMLAndConvertToJSONValid(t *testing.T) {
    testValidateYAMLAndConvertToJSON(t, "./testdata/valid.yaml")
}

func TestValidateYAMLAndConvertToJSONValidComplex(t *testing.T) {
    testValidateYAMLAndConvertToJSON(t, "./testdata/validComplex.yaml")
}

func testValidateYAMLAndConvertToJSON(t *testing.T, filePath string) {
    jsonOutput, err := ValidateYAMLAndConvertToJSON(filePath)
    if err != nil {
        t.Errorf("ValidateYAMLAndConvertToJSON failed for %s: %v", filePath, err)
        return
    }

    if jsonOutput == "" {
        t.Errorf("JSON output is empty for %s", filePath)
        return
    }

    // Convert JSON output directly back to YAML
    var jsonData interface{}
    err = json.Unmarshal([]byte(jsonOutput), &jsonData)
    if err != nil {
        t.Errorf("Failed to unmarshal JSON for %s: %v", filePath, err)
        return
    }

    yamlOutput, err := yaml.Marshal(jsonData)
    if err != nil {
        t.Errorf("Failed to marshal back to YAML for %s: %v", filePath, err)
        return
    }

    // Read the original YAML file for comparison
    originalYaml, err := ioutil.ReadFile(filePath)
    if err != nil {
        t.Errorf("Failed to read original YAML file %s: %v", filePath, err)
        return
    }

    // Unmarshal both YAMLs into interfaces for structural comparison
    var originalData, roundTripData interface{}
    errOriginal := yaml.Unmarshal(originalYaml, &originalData)
    errRoundTrip := yaml.Unmarshal(yamlOutput, &roundTripData)
    if errOriginal != nil || errRoundTrip != nil {
        t.Errorf("Failed to unmarshal YAMLs for comparison: %v, %v", errOriginal, errRoundTrip)
        return
    }

    if !reflect.DeepEqual(originalData, roundTripData) {
        t.Errorf("Round-trip YAML does not match original for %s", filePath)
    }
}
