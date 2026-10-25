package engine

import (
	"crypto/sha1"
	"encoding/hex"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"sync/atomic"
	"testing"
	"time"
)

func TestCandidates(t *testing.T) {
	got := Candidates("https://libraries.minecraft.net/com/mojang/brigadier/1.0.18/brigadier-1.0.18.jar", "bmclapi")
	want := "https://bmclapi2.bangbang93.com/maven/com/mojang/brigadier/1.0.18/brigadier-1.0.18.jar"
	if len(got) != 2 || got[0] != want {
		t.Fatalf("unexpected candidates: %v", got)
	}
	neo := Candidates("https://maven.neoforged.net/releases/net/neoforged/neoforge/21.1.265/neoforge-21.1.265-installer.jar", "bmclapi")
	if neo[0] != "https://bmclapi2.bangbang93.com/maven/net/neoforged/neoforge/21.1.265/neoforge-21.1.265-installer.jar" {
		t.Fatalf("unexpected neoforge mirror: %v", neo)
	}
	if got := Candidates("https://example.com/a.jar", "bmclapi"); len(got) != 1 {
		t.Fatalf("unknown host should not be rewritten: %v", got)
	}
	if got := Candidates("https://libraries.minecraft.net/x.jar", "official"); len(got) != 1 {
		t.Fatalf("official mirror should not be rewritten: %v", got)
	}
}

func waitDone(t *testing.T, b *Batch) Status {
	t.Helper()
	deadline := time.Now().Add(5 * time.Second)
	for time.Now().Before(deadline) {
		if s := b.Status(); s.State != Running {
			return s
		}
		time.Sleep(10 * time.Millisecond)
	}
	t.Fatal("batch did not finish")
	return Status{}
}

func TestDownloadVerifyAndSkip(t *testing.T) {
	body := []byte("hello starraft")
	sum := sha1.Sum(body)
	var hits atomic.Int32
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		hits.Add(1)
		if r.URL.Path == "/bad" {
			_, _ = w.Write([]byte("corrupted"))
			return
		}
		_, _ = w.Write(body)
	}))
	defer srv.Close()

	dir := t.TempDir()
	e := New()
	good := Task{URL: srv.URL + "/ok", Path: filepath.Join(dir, "a", "ok.bin"), SHA1: hex.EncodeToString(sum[:]), Size: int64(len(body))}
	bad := Task{URL: srv.URL + "/bad", Path: filepath.Join(dir, "bad.bin"), SHA1: hex.EncodeToString(sum[:])}

	b, err := e.Submit(Request{Tasks: []Task{good, bad}, Concurrency: 2})
	if err != nil {
		t.Fatal(err)
	}
	st := waitDone(t, b)
	if st.State != Failed || st.Completed != 1 || st.Failed != 1 {
		t.Fatalf("unexpected status: %+v", st)
	}
	if data, _ := os.ReadFile(good.Path); string(data) != string(body) {
		t.Fatalf("content mismatch: %q", data)
	}
	if _, err := os.Stat(bad.Path); !os.IsNotExist(err) {
		t.Fatal("corrupted file must not be kept")
	}

	before := hits.Load()
	b, _ = e.Submit(Request{Tasks: []Task{good}})
	if st := waitDone(t, b); st.State != Done || st.BytesDone != int64(len(body)) {
		t.Fatalf("unexpected status: %+v", st)
	}
	if hits.Load() != before {
		t.Fatal("verified file should be skipped without network access")
	}
}

func TestRejectRelativePath(t *testing.T) {
	if _, err := New().Submit(Request{Tasks: []Task{{URL: "http://x", Path: "rel/path"}}}); err == nil {
		t.Fatal("relative path must be rejected")
	}
}
