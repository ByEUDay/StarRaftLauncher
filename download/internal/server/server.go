// Package server 把下载引擎以本地 HTTP API 暴露给 BackEnd。
//
//	POST   /v1/batches        提交批次 -> {"id": "..."}
//	GET    /v1/batches        列出批次
//	GET    /v1/batches/{id}   批次状态
//	DELETE /v1/batches/{id}   取消批次
//	GET    /v1/health
package server

import (
	"crypto/subtle"
	"encoding/json"
	"net/http"

	"github.com/starraft/launcher/download/internal/engine"
)

type Server struct {
	eng   *engine.Engine
	token string
}

func New(eng *engine.Engine, token string) *Server {
	return &Server{eng: eng, token: token}
}

func (s *Server) Handler() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /v1/health", func(w http.ResponseWriter, _ *http.Request) {
		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	})
	mux.HandleFunc("POST /v1/batches", s.submit)
	mux.HandleFunc("GET /v1/batches", func(w http.ResponseWriter, _ *http.Request) {
		writeJSON(w, http.StatusOK, s.eng.List())
	})
	mux.HandleFunc("GET /v1/batches/{id}", func(w http.ResponseWriter, r *http.Request) {
		b, ok := s.eng.Get(r.PathValue("id"))
		if !ok {
			writeError(w, http.StatusNotFound, "batch not found")
			return
		}
		writeJSON(w, http.StatusOK, b.Status())
	})
	mux.HandleFunc("DELETE /v1/batches/{id}", func(w http.ResponseWriter, r *http.Request) {
		b, ok := s.eng.Get(r.PathValue("id"))
		if !ok {
			writeError(w, http.StatusNotFound, "batch not found")
			return
		}
		b.Cancel()
		w.WriteHeader(http.StatusAccepted)
	})
	return s.auth(mux)
}

func (s *Server) auth(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if s.token != "" && subtle.ConstantTimeCompare([]byte(r.Header.Get("X-StarRaft-Token")), []byte(s.token)) != 1 {
			writeError(w, http.StatusUnauthorized, "unauthorized")
			return
		}
		next.ServeHTTP(w, r)
	})
}

func (s *Server) submit(w http.ResponseWriter, r *http.Request) {
	var req engine.Request
	if err := json.NewDecoder(http.MaxBytesReader(w, r.Body, 64<<20)).Decode(&req); err != nil {
		writeError(w, http.StatusBadRequest, err.Error())
		return
	}
	b, err := s.eng.Submit(req)
	if err != nil {
		writeError(w, http.StatusBadRequest, err.Error())
		return
	}
	writeJSON(w, http.StatusCreated, map[string]string{"id": b.Status().ID})
}

func writeJSON(w http.ResponseWriter, code int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(code)
	_ = json.NewEncoder(w).Encode(v)
}

func writeError(w http.ResponseWriter, code int, msg string) {
	writeJSON(w, code, map[string]string{"error": msg})
}
