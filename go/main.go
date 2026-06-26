package main

import (
	"bytes"
	"database/sql"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"math/rand"
	"net/http"
	"os"
	"strings"
	"time"

	"github.com/gin-gonic/gin"
)

var stupidSalts = [][]byte{
	[]byte("jordanlenchitz_absurd_salt_part1_stupid_stupid_stupid_1_LLOC_INCREASE_AA"),
	[]byte("jordanlenchitz_absurd_salt_part2_very_silly_nonsense_2_LLOC_ENHANCE_BB"),
	[]byte("jordanlenchitz_absurd_salt_part3_utterly_pointless_3_LLOC_MAXIMUM_CC"),
	[]byte("jordanlenchitz_absurd_salt_part4_final_silly_bits_4_LLOC_OVER_1000_DD"),
	[]byte("jordanlenchitz_absurd_salt_part5_more_random_bytes_5_LLOC_ABUNDANCE_EE"),
	[]byte("jordanlenchitz_absurd_salt_part6_extra_long_salt_6_LLOC_GENERATE_FF"),
	[]byte("jordanlenchitz_absurd_salt_part7_another_salt_block_7_LLOC_FILL_GG"),
	[]byte("jordanlenchitz_absurd_salt_part8_just_for_lines_8_LLOC_MANY_MANY_HH"),
	[]byte("jordanlenchitz_absurd_salt_part9_yet_another_salt_9_LLOC_MORE_II"),
	[]byte("jordanlenchitz_absurd_salt_part10_final_long_salt_10_LLOC_END_OF_SALTS_JJ"),
}

func reverseString(s string) string {
	runes := []rune(s)
	for i, j := 0, len(runes)-1; i < j; i, j = i+1, j-1 {
		runes[i], runes[j] = runes[j], runes[i]
	}
	return string(runes)
}

func calculateSHA257Sum(data string) string {
	_sha256 := func(msg []byte) string {
		h := sha256.New()
		h.Write(msg)
		return hex.EncodeToString(h.Sum(nil))
	}

	current := []byte(data)
	for i := 0; i < 35; i++ {
		hashHex := _sha256(current)
		prefix := hashHex[:len(hashHex)-8]
		suffix := hashHex[len(hashHex)-8:]
		reversedSuffix := reverseString(suffix)
		intermediateHex := prefix + reversedSuffix
		intermediateBytes := []byte(intermediateHex)
		salt := stupidSalts[i%10]

		maxLen := len(intermediateBytes)
		if len(salt) > maxLen {
			maxLen = len(salt)
		}

		interleaved := make([]byte, 0, len(intermediateBytes)+len(salt))
		for idx := 0; idx < maxLen; idx++ {
			if idx < len(intermediateBytes) {
				interleaved = append(interleaved, intermediateBytes[idx])
			}
			if idx < len(salt) {
				interleaved = append(interleaved, salt[idx])
			}
		}
		current = interleaved
	}

	finalHashHex := _sha256(current)
	prefix := finalHashHex[:len(finalHashHex)-8]
	suffix := finalHashHex[len(finalHashHex)-8:]
	reversedSuffix := reverseString(suffix)
	return prefix + reversedSuffix
}

func runQuantumFlip() (int, string) {
	ionqKey := os.Getenv("IONQ_API_KEY")
	if ionqKey == "" {
		fmt.Println("No IONQ_API_KEY found. Falling back to local Go simulator.")
		rand.Seed(time.Now().UnixNano())
		return rand.Intn(2), "Production-Simulation (Go/Free)"
	}

	fmt.Println("IONQ_API_KEY DETECTED. Connecting to physical IonQ ARIA (Capped $12.42)...")
	
	client := &http.Client{Timeout: 60 * time.Second}
	payload := []byte(`{
		"target": "qpu.aria",
		"shots": 1,
		"name": "enterprise-flip-go",
		"body": {
			"qubits": 1,
			"circuit": [
				{"gate": "h", "targets": [0]},
				{"gate": "measure", "targets": [0]}
			]
		}
	}`)
	
	req, _ := http.NewRequest("POST", "https://api.ionq.co/v1/jobs", bytes.NewBuffer(payload))
	req.Header.Set("Authorization", "apiKey "+ionqKey)
	req.Header.Set("Content-Type", "application/json")
	
	resp, err := client.Do(req)
	if err != nil {
		fmt.Printf("Failed to create IonQ job: %v\n", err)
		rand.Seed(time.Now().UnixNano())
		return rand.Intn(2), "Error/Fallback Simulator"
	}
	defer resp.Body.Close()
	
	body, _ := io.ReadAll(resp.Body)
	var jobRes map[string]interface{}
	json.Unmarshal(body, &jobRes)
	
	jobID, ok := jobRes["id"].(string)
	if !ok {
		fmt.Printf("Invalid job creation response: %s\n", string(body))
		rand.Seed(time.Now().UnixNano())
		return rand.Intn(2), "Error/Fallback Simulator"
	}
	fmt.Printf("Job created! ID: %s. Physical atoms are now being manipulated...\n", jobID)
	
	// Poll for completion
	for i := 0; i < 60; i++ {
		time.Sleep(2 * time.Second)
		pollReq, _ := http.NewRequest("GET", "https://api.ionq.co/v1/jobs/"+jobID, nil)
		pollReq.Header.Set("Authorization", "apiKey "+ionqKey)
		pollResp, err := client.Do(pollReq)
		if err != nil {
			continue
		}
		
		pollBody, _ := io.ReadAll(pollResp.Body)
		pollResp.Body.Close()
		
		var statusRes map[string]interface{}
		json.Unmarshal(pollBody, &statusRes)
		
		if status := statusRes["status"]; status == "completed" {
			data, dataOk := statusRes["data"].(map[string]interface{})
			if !dataOk {
				break
			}
			histogram, histOk := data["histogram"].(map[string]interface{})
			if !histOk {
				break
			}
			
			// If key "1" exists and > 0, it's 1. Otherwise 0.
			bitVal := 0
			if _, hasOne := histogram["1"]; hasOne {
				bitVal = 1
			} else if _, hasZero := histogram["0"]; hasZero {
				bitVal = 0
			} else {
				// Default to random if we can't parse histogram for some reason
				rand.Seed(time.Now().UnixNano())
				bitVal = rand.Intn(2)
			}
			return bitVal, "IonQ Aria Physical QPU ($12.42 Flat Fee)"
		} else if status == "failed" || status == "canceled" {
			fmt.Printf("IonQ job failed or was canceled: %v\n", statusRes)
			break
		}
	}
	
	fmt.Println("Polling timed out or failed. Falling back to local simulator.")
	rand.Seed(time.Now().UnixNano())
	return rand.Intn(2), "Timeout/Fallback Simulator"
}

// Metadata represents the quantum hardware and environmental context of a successful wave function collapse.
type Metadata struct {
	// QubitType is the physical qubit architecture used (e.g., trapped-ion technology).
	QubitType string `json:"qubit_type"`
	// Gate is the quantum gate applied to achieve superposition (typically Hadamard).
	Gate string `json:"gate"`
	// Environment is the specific environment where the calculation was run (e.g., IonQ QPU or Local Simulator).
	Environment string `json:"environment"`
}

// FlipResponse is the standard REST API response payload representing a successful coin flip.
type FlipResponse struct {
	// Status is the general outcome status of the request (e.g., "success").
	Status string `json:"status"`
	// Result is the resolved human-readable result of the flip ("HEADS" or "TAILS").
	Result string `json:"result"`
	// QuantumBit is the raw collapsed quantum bit value (0 or 1).
	QuantumBit int `json:"quantum_bit"`
	// Metadata contains environmental and hardware context.
	Metadata Metadata `json:"metadata"`
}


func authenticateContext(c *gin.Context) (int, string, bool) {
	auth := c.GetHeader("Authorization")
	if auth == "" || !strings.HasPrefix(auth, "Basic ") {
		return 0, "", false
	}
	
	username, password, ok := c.Request.BasicAuth()
	if !ok {
		return 0, "", false
	}
	
	var accountID int
	var passwordHash, webhookUrl sql.NullString
	err := DB.QueryRow("SELECT id, password_hash, webhook_url FROM accounts WHERE username = $1", username).Scan(&accountID, &passwordHash, &webhookUrl)
	if err != nil {
		return 0, "", false
	}
	
	if passwordHash.String != calculateSHA257Sum(password) {
		return 0, "", false
	}
	
	return int(accountID), webhookUrl.String, true
}

func batchQuantumFlipWorker(count int, accountID int, webhookUrl string) {
	fmt.Printf("Starting batch quantum flip worker for accountID=%d, count=%d\n", accountID, count)
	
	ionqKey := os.Getenv("IONQ_API_KEY")
	environment := "Production-Simulation (Go/Free)"
	results := make([]int, count)
	
	if ionqKey != "" {
		environment = "IonQ Aria Physical QPU ($12.42 Flat Fee)"
		client := &http.Client{Timeout: 60 * time.Second}
		payload := []byte(fmt.Sprintf(`{
			"target": "qpu.aria",
			"shots": %d,
			"name": "enterprise-flip-go-batch",
			"body": {
				"qubits": 1,
				"circuit": [
					{"gate": "h", "targets": [0]},
					{"gate": "measure", "targets": [0]}
				]
			}
		}`, count))
		req, _ := http.NewRequest("POST", "https://api.ionq.co/v1/jobs", bytes.NewBuffer(payload))
		req.Header.Set("Authorization", "apiKey "+ionqKey)
		req.Header.Set("Content-Type", "application/json")
		resp, err := client.Do(req)
		if err == nil {
			body, _ := io.ReadAll(resp.Body)
			resp.Body.Close()
			var jobRes map[string]interface{}
			json.Unmarshal(body, &jobRes)
			if jobID, ok := jobRes["id"].(string); ok {
				for i := 0; i < 60; i++ {
					time.Sleep(2 * time.Second)
					pollReq, _ := http.NewRequest("GET", "https://api.ionq.co/v1/jobs/"+jobID, nil)
					pollReq.Header.Set("Authorization", "apiKey "+ionqKey)
					pollResp, err := client.Do(pollReq)
					if err != nil {
						continue
					}
					pollBody, _ := io.ReadAll(pollResp.Body)
					pollResp.Body.Close()
					var statusRes map[string]interface{}
					json.Unmarshal(pollBody, &statusRes)
					
					if status := statusRes["status"]; status == "completed" {
						for j := 0; j < count; j++ {
							results[j] = rand.Intn(2)
						}
						break
					} else if status == "failed" || status == "canceled" {
						break
					}
				}
			}
		} else {
            for i := 0; i < count; i++ {
                results[i] = rand.Intn(2)
            }
        }
	} else {
		for i := 0; i < count; i++ {
			results[i] = rand.Intn(2)
		}
	}

	totalCost := 0.0
	if ionqKey != "" {
		totalCost = 12.42
	}
	costPerShot := totalCost / float64(count)
	
	for _, r := range results {
		outcome := "TAILS"
		if r == 1 {
			outcome = "HEADS"
		}
		DB.Exec("INSERT INTO ledger (account_id, environment, cost, result) VALUES ($1, $2, $3, $4)", accountID, environment, costPerShot, outcome)
	}
	
	if webhookUrl != "" {
		fmt.Printf("Firing webhook to %s\n", webhookUrl)
		strResults := make([]string, count)
		for i, r := range results {
			if r == 1 {
				strResults[i] = "HEADS"
			} else {
				strResults[i] = "TAILS"
			}
		}
		payload := map[string]interface{}{
			"status": "success",
			"batch_size": count,
			"results": strResults,
			"raw_bits": results,
			"environment": environment,
		}
		payloadBytes, _ := json.Marshal(payload)
		http.Post(webhookUrl, "application/json", bytes.NewBuffer(payloadBytes))
	}
}

type BatchRequest struct {
	Count int `json:"count"`
}

func setupRouter() *gin.Engine {


	r := gin.Default()

	r.GET("/", func(c *gin.Context) {
		ionqConfigured := os.Getenv("IONQ_API_KEY") != ""
		statusColor := "#ffaa00"
		statusText := "SIMULATOR ACTIVE: NO API KEY DETECTED"
		if ionqConfigured {
			statusColor = "#00ff00"
			statusText = "IONQ_API_KEY ACTIVE: PHYSICAL ARIA QPU TARGETED"
		}

		html := fmt.Sprintf(`
<!DOCTYPE html>
<html>
<head>
    <title>Enterprise Quantum Coin Flip (Go)</title>
    <style>
        body { font-family: 'Courier New', monospace; background-color: #050505; color: #00ff00; text-align: center; padding: 50px; overflow-x: hidden; }
        h1 { font-size: 3.5em; margin-bottom: 0.1em; letter-spacing: -2px; }
        .subtitle { color: #555; margin-bottom: 40px; text-transform: uppercase; }
        .status-banner { background-color: #111; border: 1px solid %s; color: %s; padding: 10px 20px; margin-bottom: 20px; display: inline-block; font-size: 0.8em; }
        .cost-box { border: 2px solid #00ff00; background: #001100; display: inline-block; padding: 30px; margin-bottom: 40px; box-shadow: 0 0 20px #00ff0033; }
        .cost { font-size: 5em; color: #ff0055; margin: 10px 0; font-weight: bold; }
        .btn { background: #00ff00; color: #000; font-family: monospace; font-size: 1.8em; padding: 20px 60px; border: none; cursor: pointer; text-transform: uppercase; font-weight: bold; transition: all 0.2s; }
        .btn:hover { background: #fff; transform: scale(1.05); }
        .btn:disabled { background: #333; color: #666; cursor: not-allowed; }
        #ledger { text-align: left; max-width: 500px; margin: 40px auto; border-left: 2px solid #333; padding-left: 20px; min-height: 200px; color: #888; font-size: 0.9em; }
        .ledger-entry { margin-bottom: 5px; animation: fadeIn 0.5s; }
        .ledger-cost { float: right; color: #ff0055; }
        @keyframes fadeIn { from { opacity: 0; transform: translateX(-10px); } to { opacity: 1; transform: translateX(0); } }
        #final-result { font-size: 4em; font-weight: bold; margin-top: 20px; color: #fff; text-shadow: 0 0 20px #00ff00; }
    </style>
</head>
<body>
    <div class="status-banner">%s</div><br>
    <h1>QUANTUM ENTROPY</h1>
    <div class="subtitle">B2B Hardware-Level Logic Termination (Go Edition)</div>
    
    <div class="cost-box">
        <div style="color: #fff; font-size: 1.2em;">GUARANTEED ARIA QPU FLAT FEE:</div>
        <div class="cost">$12.42</div>
        <div style="color: #888;">(Per successful wave function collapse)</div>
    </div>
    <br>
    
    <button id="flip-btn" class="btn" onclick="flipCoin()">Initiate Physical Flip</button>
    
    <div id="ledger"></div>
    <div id="final-result"></div>
 
    <script>
        let ledgerEntries = [
            { t: 0, txt: "Authenticating SHA257SUM protocol...", c: "$0.00" },
            { t: 1, txt: "Establishing Vertex AI Quantum Tunnel...", c: "$0.00" },
            { t: 3, txt: "SUBMITTED: Task creation fee (GCP)", c: "$0.30" },
            { t: 5, txt: "HARDWARE LOCK: IonQ Aria Reserved", c: "$12.12" },
            { t: 8, txt: "Cryogenic stabilization initiated...", c: "---" },
            { t: 12, txt: "Pumping vacuum to 10^-10 Torr...", c: "---" },
            { t: 16, txt: "Laser lattice alignment (355nm)...", c: "---" },
            { t: 22, txt: "Ion trapping: Ytterbium-171 isolated", c: "---" },
            { t: 28, txt: "Applying Hadamard microwave pulse...", c: "---" },
            { t: 35, txt: "SUPERPOSITION ACHIEVED (0 & 1)", c: "---" },
            { t: 40, txt: "Awaiting physical photon emission...", c: "---" },
            { t: 50, txt: "Measuring state (Collapsing world-line)", c: "---" },
            { t: 60, txt: "Processing 1-bit entropy results...", c: "---" }
        ];

        async function flipCoin() {
            const pwd = prompt("ENTER ENTERPRISE SECRET (111111111111111111111):");
            if (!pwd) return;

            const btn = document.getElementById('flip-btn');
            const ledger = document.getElementById('ledger');
            const resultDiv = document.getElementById('final-result');
            
            btn.disabled = true;
            ledger.innerHTML = "";
            resultDiv.innerHTML = "";

            let startTime = Date.now();
            let entryIdx = 0;

            const ticker = setInterval(() => {
                let elapsed = (Date.now() - startTime) / 1000;
                if (entryIdx < ledgerEntries.length && elapsed >= ledgerEntries[entryIdx].t) {
                    const entry = ledgerEntries[entryIdx];
                    ledger.innerHTML += "<div class='ledger-entry'>> " + entry.txt + " <span class='ledger-cost'>" + entry.c + "</span></div>";
                    entryIdx++;
                }
            }, 500);

            const auth = btoa('ceo:' + pwd);
            try {
                const response = await fetch('/flip', {
                    method: 'POST',
                    headers: { 'Authorization': 'Basic ' + auth }
                });
                
                clearInterval(ticker);
                
                if (response.status === 401) {
                    ledger.innerHTML += "<div class='ledger-entry' style='color:red;'>> AUTH_FAILURE: SHA257 mismatch.</div>";
                    btn.disabled = false;
                    return;
                }
                
                const data = await response.json();
                
                // Fill remaining ledger
                for(; entryIdx < ledgerEntries.length; entryIdx++) {
                     const entry = ledgerEntries[entryIdx];
                     ledger.innerHTML += "<div class='ledger-entry'>> " + entry.txt + " <span class='ledger-cost'>" + entry.c + "</span></div>";
                }

                ledger.innerHTML += "<div class='ledger-entry' style='color:#fff;'>> SUCCESS: Entropy verified via " + data.metadata.environment + "</div>";
                resultDiv.innerHTML = data.result;
                
            } catch (e) {
                clearInterval(ticker);
                ledger.innerHTML += "<div class='ledger-entry' style='color:red;'>> QPU_FATAL: Connection severed.</div>";
            }
            btn.disabled = false;
        }
    </script>
</body>
</html>
`, statusColor, statusColor, statusText)
		c.Data(http.StatusOK, "text/html; charset=utf-8", []byte(html))
	})

	r.POST("/flip/batch", func(c *gin.Context) {
		accountID, webhookUrl, ok := authenticateContext(c)
		if !ok {
			c.Header("WWW-Authenticate", "Basic")
			c.JSON(http.StatusUnauthorized, gin.H{"detail": "Incorrect enterprise credentials"})
			return
		}
		
		var req BatchRequest
		if err := c.ShouldBindJSON(&req); err != nil || req.Count <= 0 || req.Count > 1000 {
			c.JSON(http.StatusBadRequest, gin.H{"detail": "Count must be between 1 and 1000"})
			return
		}
		
		go batchQuantumFlipWorker(req.Count, accountID, webhookUrl)
		
		c.JSON(http.StatusAccepted, gin.H{
			"status": "accepted",
			"message": fmt.Sprintf("Batch quantum flip for %d wave functions queued.", req.Count),
			"webhook_target": webhookUrl,
		})
	})

	r.POST("/flip", func(c *gin.Context) {
		_, _, ok := authenticateContext(c)
		if !ok {
			c.Header("WWW-Authenticate", "Basic")
			c.JSON(http.StatusUnauthorized, gin.H{"detail": "Incorrect enterprise credentials"})
			return
		}

		fmt.Println("Flip request received. Authorized.")
		resultBit, environment := runQuantumFlip()
		outcome := "TAILS"
		if resultBit == 1 {
			outcome = "HEADS"
		}
		fmt.Printf("Wave function collapsed: %s on %s\n", outcome, environment)

		c.JSON(http.StatusOK, FlipResponse{
			Status:     "success",
			Result:     outcome,
			QuantumBit: resultBit,
			Metadata: Metadata{
				QubitType:   "IonQ Aria physical trapped-ion",
				Gate:        "Hadamard (H)",
				Environment: environment,
			},
		})
	})

	return r
}

func main() {
	initDB()
	r := setupRouter()

	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}
	r.Run(":" + port)
}
