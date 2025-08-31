package codex

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestEmbed(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/v1/embeddings" {
			t.Fatalf("unexpected path: %s", r.URL.Path)
		}
		var req struct {
			Texts []string `json:"texts"`
		}
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			t.Fatalf("decode: %v", err)
		}
		if len(req.Texts) != 1 || req.Texts[0] != "hi" {
			t.Fatalf("bad request: %+v", req)
		}
		json.NewEncoder(w).Encode(map[string]any{"embeddings": [][]float32{{1, 2, 3}}})
	}))
	defer srv.Close()
	c := NewClient(srv.URL)
	emb, err := c.Embed(context.Background(), []string{"hi"})
	if err != nil {
		t.Fatalf("Embed: %v", err)
	}
	if len(emb) != 1 || len(emb[0]) != 3 {
		t.Fatalf("unexpected embeddings: %v", emb)
	}
}

func TestUpsert(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/v1/vector/upsert" {
			t.Fatalf("unexpected path: %s", r.URL.Path)
		}
		var req struct {
			Vectors []VectorRecord `json:"vectors"`
		}
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			t.Fatalf("decode: %v", err)
		}
		if len(req.Vectors) != 1 || req.Vectors[0].ID != 1 {
			t.Fatalf("bad request: %+v", req)
		}
		json.NewEncoder(w).Encode(map[string]any{"inserted": 1})
	}))
	defer srv.Close()
	c := NewClient(srv.URL)
	n, err := c.Upsert(context.Background(), []VectorRecord{{ID: 1, Values: []float32{1, 2}, Document: "d"}})
	if err != nil {
		t.Fatalf("Upsert: %v", err)
	}
	if n != 1 {
		t.Fatalf("expected 1 inserted, got %d", n)
	}
}

func TestQuery(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/v1/vector/query" {
			t.Fatalf("unexpected path: %s", r.URL.Path)
		}
		json.NewEncoder(w).Encode(map[string]any{
			"results": []Reference{{ID: 1, Document: "doc"}},
		})
	}))
	defer srv.Close()
	c := NewClient(srv.URL)
	res, err := c.Query(context.Background(), []float32{1, 2}, 1)
	if err != nil {
		t.Fatalf("Query: %v", err)
	}
	if len(res) != 1 || res[0].Document != "doc" {
		t.Fatalf("unexpected result: %v", res)
	}
}

func TestChat(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/v1/chat" {
			t.Fatalf("unexpected path: %s", r.URL.Path)
		}
		json.NewEncoder(w).Encode(map[string]any{"reply": "ok"})
	}))
	defer srv.Close()
	c := NewClient(srv.URL)
	msg := []Message{{Role: "user", Content: "hi"}}
	reply, err := c.Chat(context.Background(), "", msg)
	if err != nil {
		t.Fatalf("Chat: %v", err)
	}
	if reply != "ok" {
		t.Fatalf("unexpected reply: %s", reply)
	}
}

func TestRAGAnswer(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/v1/rag/answer" {
			t.Fatalf("unexpected path: %s", r.URL.Path)
		}
		json.NewEncoder(w).Encode(map[string]any{
			"answer":   "42",
			"contexts": []string{"doc"},
		})
	}))
	defer srv.Close()
	c := NewClient(srv.URL)
	res, err := c.RAGAnswer(context.Background(), "?", 1, "", false)
	if err != nil {
		t.Fatalf("RAGAnswer: %v", err)
	}
	if res.Answer != "42" || len(res.References) != 1 {
		t.Fatalf("unexpected result: %v", res)
	}
}

func ExampleClient_Chat() {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		json.NewEncoder(w).Encode(map[string]any{"reply": "hello"})
	}))
	defer srv.Close()
	c := NewClient(srv.URL)
	msg := []Message{{Role: "user", Content: "hi"}}
	reply, _ := c.Chat(context.Background(), "", msg)
	fmt.Println(reply)
	// Output: hello
}
