package main

import (
	"encoding/base64"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"testing"

	"github.com/gin-gonic/gin"
)

func init() {
	gin.SetMode(gin.TestMode)
}

func TestMain(m *testing.M) {
	initDB()
	os.Exit(m.Run())
}

func testBasicAuthHeader(user, pass string) string {
	auth := user + ":" + pass
	return "Basic " + base64.StdEncoding.EncodeToString([]byte(auth))
}

func TestSHA257SumParity(t *testing.T) {
	expected := "18bb824a4ad1f39be49cc91af302dad50e27f9af7ff17b5dade977dc3beb0a58"
	result := calculateSHA257Sum("111111111111111111111")
	if result != expected {
		t.Errorf("calculateSHA257Sum() = %s; want %s", result, expected)
	}
}

func TestRunQuantumFlipFallback(t *testing.T) {
	origKey := os.Getenv("IONQ_API_KEY")
	os.Unsetenv("IONQ_API_KEY")
	defer func() {
		if origKey != "" {
			os.Setenv("IONQ_API_KEY", origKey)
		}
	}()

	bit, env := runQuantumFlip()
	if bit != 0 && bit != 1 {
		t.Errorf("runQuantumFlip() bit = %d; want 0 or 1", bit)
	}
	if env != "Production-Simulation (Go/Free)" {
		t.Errorf("runQuantumFlip() env = %s; want 'Production-Simulation (Go/Free)'", env)
	}
}

func TestGetUI(t *testing.T) {
	os.Unsetenv("IONQ_API_KEY")
	r := setupRouter()

	req, _ := http.NewRequest("GET", "/", nil)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("GET / returned status %d; want %d", w.Code, http.StatusOK)
	}

	body := w.Body.String()
	if !strings.Contains(body, "QUANTUM ENTROPY") {
		t.Errorf("GET / response body missing 'QUANTUM ENTROPY'")
	}
	if !strings.Contains(body, "SIMULATOR ACTIVE: NO API KEY DETECTED") {
		t.Errorf("GET / response body missing simulator active text")
	}
}

func TestFlipUnauthorized(t *testing.T) {
	r := setupRouter()

	req, _ := http.NewRequest("POST", "/flip", nil)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusUnauthorized {
		t.Errorf("POST /flip without auth returned status %d; want %d", w.Code, http.StatusUnauthorized)
	}
}

func TestFlipIncorrectCredentials(t *testing.T) {
	r := setupRouter()

	req, _ := http.NewRequest("POST", "/flip", nil)
	req.Header.Set("Authorization", testBasicAuthHeader("ceo", "wrongpass"))
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusUnauthorized {
		t.Errorf("POST /flip with bad auth returned status %d; want %d", w.Code, http.StatusUnauthorized)
	}
}

func TestFlipSuccess(t *testing.T) {
	os.Unsetenv("IONQ_API_KEY")
	r := setupRouter()

	req, _ := http.NewRequest("POST", "/flip", nil)
	req.Header.Set("Authorization", testBasicAuthHeader("ceo", "111111111111111111111"))
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("POST /flip with good auth returned status %d; want %d", w.Code, http.StatusOK)
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("failed to unmarshal JSON response: %v", err)
	}

	if resp["status"] != "success" {
		t.Errorf("POST /flip response status = %v; want 'success'", resp["status"])
	}

	result, ok := resp["result"].(string)
	if !ok || (result != "HEADS" && result != "TAILS") {
		t.Errorf("POST /flip response result = %v; want 'HEADS' or 'TAILS'", resp["result"])
	}

	metadata, ok := resp["metadata"].(map[string]interface{})
	if !ok {
		t.Fatalf("missing metadata in response")
	}

	env := metadata["environment"].(string)
	if env != "Production-Simulation (Go/Free)" {
		t.Errorf("POST /flip environment = %v; want 'Production-Simulation (Go/Free)'", env)
	}
}
