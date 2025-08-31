package codex

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
)

// Client wraps HTTP access to the Codex server API.
type Client struct {
	BaseURL    string
	HTTPClient *http.Client
}

// NewClient creates a new Client with the given baseURL.
func NewClient(baseURL string) *Client {
	return &Client{BaseURL: strings.TrimRight(baseURL, "/"), HTTPClient: &http.Client{}}
}

// Message represents a chat message.
type Message struct {
	Role    string `json:"role"`
	Content string `json:"content"`
}

// Reference represents a document reference returned by the server.
type Reference struct {
	ID       uint32 `json:"id,omitempty"`
	Document string `json:"document"`
}

// Result represents a RAG answer with supporting references.
type Result struct {
	Answer     string      `json:"answer"`
	References []Reference `json:"references"`
}

// VectorRecord represents a vector stored in the vector database.
type VectorRecord struct {
	ID       uint32    `json:"id"`
	Values   []float32 `json:"values"`
	Document string    `json:"document"`
}

func (c *Client) do(ctx context.Context, method, path string, reqBody, respBody interface{}) error {
	var body io.Reader
	if reqBody != nil {
		b, err := json.Marshal(reqBody)
		if err != nil {
			return err
		}
		body = bytes.NewReader(b)
	}
	req, err := http.NewRequestWithContext(ctx, method, c.BaseURL+path, body)
	if err != nil {
		return err
	}
	if reqBody != nil {
		req.Header.Set("Content-Type", "application/json")
	}
	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		b, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("http %d: %s", resp.StatusCode, strings.TrimSpace(string(b)))
	}
	if respBody != nil {
		return json.NewDecoder(resp.Body).Decode(respBody)
	}
	io.Copy(io.Discard, resp.Body)
	return nil
}

// Embed sends texts for embedding and returns their vector representations.
func (c *Client) Embed(ctx context.Context, texts []string) ([][]float32, error) {
	req := struct {
		Texts []string `json:"texts"`
	}{Texts: texts}
	var resp struct {
		Embeddings [][]float32 `json:"embeddings"`
	}
	if err := c.do(ctx, http.MethodPost, "/v1/embeddings", &req, &resp); err != nil {
		return nil, err
	}
	return resp.Embeddings, nil
}

// Upsert inserts vectors into the database and returns the count inserted.
func (c *Client) Upsert(ctx context.Context, vectors []VectorRecord) (int, error) {
	req := struct {
		Vectors []VectorRecord `json:"vectors"`
	}{Vectors: vectors}
	var resp struct {
		Inserted int `json:"inserted"`
	}
	if err := c.do(ctx, http.MethodPost, "/v1/vector/upsert", &req, &resp); err != nil {
		return 0, err
	}
	return resp.Inserted, nil
}

// Query searches the vector database and returns matching references.
func (c *Client) Query(ctx context.Context, vector []float32, topK int) ([]Reference, error) {
	req := struct {
		Vector []float32 `json:"vector"`
		TopK   int       `json:"top_k"`
	}{Vector: vector, TopK: topK}
	var resp struct {
		Results []Reference `json:"results"`
	}
	if err := c.do(ctx, http.MethodPost, "/v1/vector/query", &req, &resp); err != nil {
		return nil, err
	}
	return resp.Results, nil
}

// Chat performs a chat completion using the provided messages.
// Tier may be empty to use the default.
func (c *Client) Chat(ctx context.Context, tier string, messages []Message) (string, error) {
	req := struct {
		Tier     *string   `json:"tier,omitempty"`
		Messages []Message `json:"messages"`
	}{Messages: messages}
	if tier != "" {
		req.Tier = &tier
	}
	var resp struct {
		Reply string `json:"reply"`
	}
	if err := c.do(ctx, http.MethodPost, "/v1/chat", &req, &resp); err != nil {
		return "", err
	}
	return resp.Reply, nil
}

// RAGAnswer performs a retrieval-augmented generation request.
func (c *Client) RAGAnswer(ctx context.Context, question string, topK int, tier string, translate bool) (Result, error) {
	req := struct {
		Question  string  `json:"question"`
		TopK      int     `json:"top_k"`
		Tier      *string `json:"tier,omitempty"`
		Translate *bool   `json:"translate,omitempty"`
	}{Question: question, TopK: topK}
	if tier != "" {
		req.Tier = &tier
	}
	if translate {
		req.Translate = &translate
	}
	var resp struct {
		Answer   string   `json:"answer"`
		Contexts []string `json:"contexts"`
	}
	if err := c.do(ctx, http.MethodPost, "/v1/rag/answer", &req, &resp); err != nil {
		return Result{}, err
	}
	refs := make([]Reference, len(resp.Contexts))
	for i, doc := range resp.Contexts {
		refs[i] = Reference{Document: doc}
	}
	return Result{Answer: resp.Answer, References: refs}, nil
}
