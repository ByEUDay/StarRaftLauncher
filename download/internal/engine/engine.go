// Package engine 实现并发、可重试、带 SHA-1 校验与镜像回退的批量下载。
package engine

import (
	"context"
	"crypto/sha1"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"sync"
	"sync/atomic"
	"time"
)

const (
	UserAgent          = "StarRaftLauncher-srdl/0.1"
	defaultConcurrency = 16
	maxConcurrency     = 64
	attemptsPerURL     = 3
	keepFinished       = 32
)

type Task struct {
	URL  string `json:"url"`
	Path string `json:"path"`
	SHA1 string `json:"sha1,omitempty"`
	Size int64  `json:"size,omitempty"`
}

type Request struct {
	Label       string `json:"label"`
	Tasks       []Task `json:"tasks"`
	Mirror      string `json:"mirror"`
	Concurrency int    `json:"concurrency"`
}

type State string

const (
	Running   State = "running"
	Done      State = "done"
	Failed    State = "failed"
	Cancelled State = "cancelled"
)

type Status struct {
	ID         string     `json:"id"`
	Label      string     `json:"label"`
	State      State      `json:"state"`
	Total      int        `json:"total"`
	Completed  int64      `json:"completed"`
	Failed     int64      `json:"failed"`
	BytesDone  int64      `json:"bytesDone"`
	BytesTotal int64      `json:"bytesTotal"`
	Errors     []string   `json:"errors"`
	StartedAt  time.Time  `json:"startedAt"`
	FinishedAt *time.Time `json:"finishedAt,omitempty"`
}

type Batch struct {
	id, label   string
	mirror      string
	tasks       []Task
	concurrency int
	ctx         context.Context
	cancel      context.CancelFunc

	completed, failed, bytesDone atomic.Int64
	bytesTotal                   int64
	startedAt                    time.Time

	mu         sync.Mutex
	state      State
	errors     []string
	finishedAt *time.Time
}

func (b *Batch) Status() Status {
	b.mu.Lock()
	defer b.mu.Unlock()
	errs := make([]string, len(b.errors))
	copy(errs, b.errors)
	return Status{
		ID: b.id, Label: b.label, State: b.state, Total: len(b.tasks),
		Completed: b.completed.Load(), Failed: b.failed.Load(),
		BytesDone: b.bytesDone.Load(), BytesTotal: b.bytesTotal,
		Errors: errs, StartedAt: b.startedAt, FinishedAt: b.finishedAt,
	}
}

func (b *Batch) Cancel() { b.cancel() }

func (b *Batch) addError(msg string) {
	b.mu.Lock()
	defer b.mu.Unlock()
	if len(b.errors) < 50 {
		b.errors = append(b.errors, msg)
	}
}

type Engine struct {
	client *http.Client
	seq    atomic.Int64
	// 同一目标路径的下载互斥（多个批次可能同时安装同一游戏版本）
	pathLocks sync.Map

	mu      sync.RWMutex
	batches map[string]*Batch
}

func New() *Engine {
	transport := &http.Transport{
		Proxy:                 http.ProxyFromEnvironment,
		MaxIdleConns:          128,
		MaxIdleConnsPerHost:   32,
		IdleConnTimeout:       90 * time.Second,
		TLSHandshakeTimeout:   15 * time.Second,
		ResponseHeaderTimeout: 30 * time.Second,
		ForceAttemptHTTP2:     true,
	}
	return &Engine{client: &http.Client{Transport: transport}, batches: map[string]*Batch{}}
}

func (e *Engine) Submit(req Request) (*Batch, error) {
	if len(req.Tasks) == 0 {
		return nil, errors.New("no tasks")
	}
	conc := req.Concurrency
	if conc <= 0 {
		conc = defaultConcurrency
	}
	if conc > maxConcurrency {
		conc = maxConcurrency
	}

	// 按目标路径去重，并要求绝对路径，避免写到意外位置
	seen := map[string]bool{}
	tasks := make([]Task, 0, len(req.Tasks))
	var total int64
	for _, t := range req.Tasks {
		if t.URL == "" || !filepath.IsAbs(t.Path) {
			return nil, fmt.Errorf("invalid task: url=%q path=%q", t.URL, t.Path)
		}
		clean := filepath.Clean(t.Path)
		if seen[clean] {
			continue
		}
		seen[clean] = true
		t.Path = clean
		tasks = append(tasks, t)
		total += t.Size
	}

	ctx, cancel := context.WithCancel(context.Background())
	b := &Batch{
		id:    fmt.Sprintf("b%d-%d", time.Now().Unix(), e.seq.Add(1)),
		label: req.Label, mirror: req.Mirror, tasks: tasks, concurrency: conc,
		ctx: ctx, cancel: cancel, bytesTotal: total, startedAt: time.Now(), state: Running,
	}

	e.mu.Lock()
	e.batches[b.id] = b
	e.pruneLocked()
	e.mu.Unlock()

	go e.run(b)
	return b, nil
}

func (e *Engine) Get(id string) (*Batch, bool) {
	e.mu.RLock()
	defer e.mu.RUnlock()
	b, ok := e.batches[id]
	return b, ok
}

func (e *Engine) List() []Status {
	e.mu.RLock()
	defer e.mu.RUnlock()
	out := make([]Status, 0, len(e.batches))
	for _, b := range e.batches {
		out = append(out, b.Status())
	}
	sort.Slice(out, func(i, j int) bool { return out[i].StartedAt.After(out[j].StartedAt) })
	return out
}

// pruneLocked 只保留最近的若干个已结束批次
func (e *Engine) pruneLocked() {
	var finished []*Batch
	for _, b := range e.batches {
		b.mu.Lock()
		if b.state != Running {
			finished = append(finished, b)
		}
		b.mu.Unlock()
	}
	if len(finished) <= keepFinished {
		return
	}
	sort.Slice(finished, func(i, j int) bool { return finished[i].startedAt.Before(finished[j].startedAt) })
	for _, b := range finished[:len(finished)-keepFinished] {
		delete(e.batches, b.id)
	}
}

func (e *Engine) run(b *Batch) {
	sem := make(chan struct{}, b.concurrency)
	var wg sync.WaitGroup

loop:
	for _, t := range b.tasks {
		select {
		case <-b.ctx.Done():
			break loop
		case sem <- struct{}{}:
		}
		wg.Add(1)
		go func(t Task) {
			defer wg.Done()
			defer func() { <-sem }()
			if err := e.fetch(b, t); err != nil {
				b.failed.Add(1)
				b.addError(fmt.Sprintf("%s: %v", filepath.Base(t.Path), err))
			} else {
				b.completed.Add(1)
			}
		}(t)
	}
	wg.Wait()

	now := time.Now()
	b.mu.Lock()
	switch {
	case b.ctx.Err() != nil:
		b.state = Cancelled
	case b.failed.Load() > 0:
		b.state = Failed
	default:
		b.state = Done
	}
	b.finishedAt = &now
	b.mu.Unlock()
	b.cancel()
}

type httpError struct{ code int }

func (h *httpError) Error() string { return fmt.Sprintf("HTTP %d", h.code) }

func (e *Engine) fetch(b *Batch, t Task) error {
	lock, _ := e.pathLocks.LoadOrStore(t.Path, &sync.Mutex{})
	lock.(*sync.Mutex).Lock()
	defer lock.(*sync.Mutex).Unlock()

	if ok, _ := Verify(t); ok {
		b.bytesDone.Add(t.Size)
		return nil
	}
	if err := os.MkdirAll(filepath.Dir(t.Path), 0o755); err != nil {
		return err
	}

	var lastErr error
	for _, url := range Candidates(t.URL, b.mirror) {
		for attempt := 0; attempt < attemptsPerURL; attempt++ {
			if err := b.ctx.Err(); err != nil {
				return err
			}
			if attempt > 0 {
				time.Sleep(time.Duration(attempt) * 600 * time.Millisecond)
			}
			n, err := e.downloadOnce(b.ctx, url, t, &b.bytesDone)
			if err == nil {
				return nil
			}
			b.bytesDone.Add(-n)
			lastErr = err
			var he *httpError
			if errors.As(err, &he) && he.code >= 400 && he.code < 500 {
				break // 4xx 不重试，直接换下一个源
			}
		}
	}
	return lastErr
}

type countingWriter struct {
	counter *atomic.Int64
	track   bool
	n       int64
}

func (c *countingWriter) Write(p []byte) (int, error) {
	if c.track {
		c.counter.Add(int64(len(p)))
		c.n += int64(len(p))
	}
	return len(p), nil
}

// downloadOnce 下载到 .part 临时文件，校验通过后原子重命名。返回计入进度的字节数。
func (e *Engine) downloadOnce(ctx context.Context, url string, t Task, counter *atomic.Int64) (int64, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return 0, err
	}
	req.Header.Set("User-Agent", UserAgent)

	resp, err := e.client.Do(req)
	if err != nil {
		return 0, err
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return 0, &httpError{resp.StatusCode}
	}

	tmp := t.Path + ".part"
	f, err := os.Create(tmp)
	if err != nil {
		return 0, err
	}
	h := sha1.New()
	cw := &countingWriter{counter: counter, track: t.Size > 0}
	n, err := io.Copy(io.MultiWriter(f, h, cw), resp.Body)
	if cerr := f.Close(); err == nil {
		err = cerr
	}
	fail := func(e error) (int64, error) {
		_ = os.Remove(tmp)
		return cw.n, e
	}
	if err != nil {
		return fail(err)
	}
	if t.Size > 0 && n != t.Size {
		return fail(fmt.Errorf("size mismatch: got %d, want %d", n, t.Size))
	}
	if t.SHA1 != "" && !strings.EqualFold(hex.EncodeToString(h.Sum(nil)), t.SHA1) {
		return fail(errors.New("sha1 mismatch"))
	}
	if err := os.Rename(tmp, t.Path); err != nil {
		return fail(err)
	}
	return cw.n, nil
}

// Verify 检查本地文件是否已经完整：有 SHA-1 则校验哈希，否则比较大小；两者都没有时以存在为准。
func Verify(t Task) (bool, error) {
	info, err := os.Stat(t.Path)
	if err != nil || info.IsDir() {
		return false, err
	}
	if t.Size > 0 && info.Size() != t.Size {
		return false, nil
	}
	if t.SHA1 == "" {
		return true, nil
	}
	f, err := os.Open(t.Path)
	if err != nil {
		return false, err
	}
	defer f.Close()
	h := sha1.New()
	if _, err := io.Copy(h, f); err != nil {
		return false, err
	}
	return strings.EqualFold(hex.EncodeToString(h.Sum(nil)), t.SHA1), nil
}
