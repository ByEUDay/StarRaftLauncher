using System.Diagnostics;
using System.Security.Cryptography;

namespace StarRaft.Desktop;

/// <summary>
/// 启动并托管 Rust BackEnd 进程：生成会话令牌、等待 "LISTEN &lt;port&gt;" 握手、转存日志，
/// 退出时关闭 stdin 让后端优雅结束（后端随之关闭下载引擎与 JVM 侧车）。
/// </summary>
public sealed class BackendHost : IDisposable
{
    private readonly AppPaths _paths;
    private Process? _process;
    private StreamWriter? _log;

    public string Token { get; } = Convert.ToHexString(RandomNumberGenerator.GetBytes(24)).ToLowerInvariant();
    public int Port { get; private set; }

    public BackendHost(AppPaths paths) => _paths = paths;

    /// <returns>前端入口 URL（已附带 API 地址与令牌）</returns>
    public string Start(TimeSpan timeout)
    {
        if (!File.Exists(_paths.BackendExe))
            throw new FileNotFoundException($"找不到后端程序：{_paths.BackendExe}");

        var psi = new ProcessStartInfo(_paths.BackendExe)
        {
            UseShellExecute = false,
            CreateNoWindow = true,
            RedirectStandardInput = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            WorkingDirectory = _paths.Home,
        };
        psi.Environment["STARRAFT_TOKEN"] = Token;
        psi.Environment["STARRAFT_HOME"] = _paths.Home;
        psi.Environment["STARRAFT_DATA_DIR"] = _paths.DataDir;
        psi.Environment["STARRAFT_WATCH_STDIN"] = "1";
        if (_paths.DownloadBin is { } srdl) psi.Environment["STARRAFT_DOWNLOAD_BIN"] = srdl;

        _log = new StreamWriter(Path.Combine(_paths.LogDir, "backend.log"), append: false) { AutoFlush = true };
        _process = Process.Start(psi) ?? throw new InvalidOperationException("后端进程启动失败");

        var log = _log;
        _process.ErrorDataReceived += (_, e) =>
        {
            if (e.Data is null) return;
            lock (log) log.WriteLine(e.Data);
        };
        _process.BeginErrorReadLine();

        var stdout = _process.StandardOutput;
        var handshake = Task.Run(() =>
        {
            while (stdout.ReadLine() is { } line)
            {
                if (line.StartsWith("LISTEN ", StringComparison.Ordinal))
                    return int.Parse(line.AsSpan(7).Trim());
            }
            throw new InvalidOperationException("后端在握手前退出，详见 logs/backend.log");
        });

        if (!handshake.Wait(timeout))
            throw new TimeoutException("等待后端启动超时");
        Port = handshake.Result;

        // 继续排空 stdout，避免管道写满阻塞后端
        _ = Task.Run(() => { while (stdout.ReadLine() is not null) { } });

        var api = $"http://127.0.0.1:{Port}";
        return _paths.DevUrl is { Length: > 0 } dev
            ? $"{dev.TrimEnd('/')}/?api={Uri.EscapeDataString(api)}&token={Token}"
            : $"{api}/?token={Token}";
    }

    public void Dispose()
    {
        if (_process is { HasExited: false } p)
        {
            try
            {
                p.StandardInput.Close();
                if (!p.WaitForExit(5000)) p.Kill(entireProcessTree: true);
            }
            catch (Exception e)
            {
                Console.Error.WriteLine($"stopping backend: {e.Message}");
            }
        }
        _process?.Dispose();
        _log?.Dispose();
    }
}
