// srdl —— StarRaftLauncher 下载引擎
//
// 作为本地 HTTP 守护进程运行，启动后向 stdout 输出 `LISTEN <port>` 完成握手；
// stdin 关闭（父进程退出）时自动退出。
package main

import (
	"flag"
	"fmt"
	"io"
	"log"
	"net"
	"net/http"
	"os"

	"github.com/starraft/launcher/download/internal/engine"
	"github.com/starraft/launcher/download/internal/server"
)

func main() {
	port := flag.Int("port", 0, "listen port (0 = random)")
	watchStdin := flag.Bool("watch-stdin", true, "exit when stdin is closed")
	flag.Parse()

	log.SetOutput(os.Stderr)
	log.SetFlags(log.LstdFlags | log.Lmsgprefix)
	log.SetPrefix("srdl: ")

	ln, err := net.Listen("tcp", fmt.Sprintf("127.0.0.1:%d", *port))
	if err != nil {
		log.Fatalf("listen: %v", err)
	}

	srv := server.New(engine.New(), os.Getenv("STARRAFT_DL_TOKEN"))

	fmt.Printf("LISTEN %d\n", ln.Addr().(*net.TCPAddr).Port)

	if *watchStdin {
		go func() {
			_, _ = io.Copy(io.Discard, os.Stdin)
			log.Println("stdin closed, exiting")
			os.Exit(0)
		}()
	}

	if err := http.Serve(ln, srv.Handler()); err != nil {
		log.Fatalf("serve: %v", err)
	}
}
