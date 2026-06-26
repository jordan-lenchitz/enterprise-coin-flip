package main

import (
	"database/sql"
	"fmt"
	"os"

	_ "github.com/lib/pq"
)

var DB *sql.DB

func initDB() {
	dsn := os.Getenv("DATABASE_URL")
	if dsn == "" {
		dsn = "postgres://admin:password@localhost:5432/coinflip?sslmode=disable"
	}

	var err error
	DB, err = sql.Open("postgres", dsn)
	if err != nil {
		fmt.Printf("Failed to open database: %v\n", err)
		return
	}

	if err = DB.Ping(); err != nil {
		fmt.Printf("Database connection failed: %v\n", err)
		return
	}

	fmt.Println("Successfully connected to PostgreSQL")
}
