using System.Diagnostics;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text.Json;
using Photino.NET;

namespace StarRaft.Desktop;

/// <summary>
/// 前端 ↔ 宿主的原生能力桥（Photino web message）。
/// 请求：{"id":"1","type":"pickFolder","payload":{...}}
/// 响应：{"id":"1","ok":true,"result":...} / {"id":"1","ok":false,"error":"..."}
/// </summary>
public sealed class NativeBridge
{
    private static readonly JsonSerializerOptions Json = new(JsonSerializerDefaults.Web);
    private readonly PhotinoWindow _window;
    private readonly AppPaths _paths;

    public NativeBridge(PhotinoWindow window, AppPaths paths)
    {
        _window = window;
        _paths = paths;
    }

    private sealed record Request(string Id, string Type, JsonElement Payload);

    public void OnMessage(object? sender, string raw)
    {
        Request? req;
        try { req = JsonSerializer.Deserialize<Request>(raw, Json); }
        catch (JsonException) { return; }
        if (req is null) return;

        object? result = null;
        string? error = null;
        try { result = Handle(req); }
        catch (Exception e) { error = e.Message; }

        _window.SendWebMessage(JsonSerializer.Serialize(new { id = req.Id, ok = error is null, result, error }, Json));
    }

    private object? Handle(Request req) => req.Type switch
    {
        "pickFolder" => _window.ShowOpenFolder(Arg(req, "title") ?? "选择文件夹")?.FirstOrDefault(),
        "pickFile" => _window.ShowOpenFile(Arg(req, "title") ?? "选择文件")?.FirstOrDefault(),
        "openPath" => OpenPath(Arg(req, "path") ?? throw new ArgumentException("缺少 path")),
        "openUrl" => OpenUrl(Arg(req, "url") ?? throw new ArgumentException("缺少 url")),
        "hostInfo" => new
        {
            version = Assembly.GetExecutingAssembly().GetName().Version?.ToString(3),
            os = RuntimeInformation.OSDescription,
            runtime = RuntimeInformation.FrameworkDescription,
            dataDir = _paths.DataDir,
            logDir = _paths.LogDir,
        },
        "setTitle" => SetTitle(Arg(req, "title") ?? "StarRaft Launcher"),
        _ => throw new NotSupportedException($"未知的桥接调用：{req.Type}"),
    };

    private static string? Arg(Request req, string name) =>
        req.Payload.ValueKind == JsonValueKind.Object && req.Payload.TryGetProperty(name, out var v) && v.ValueKind == JsonValueKind.String
            ? v.GetString()
            : null;

    private object? SetTitle(string title)
    {
        _window.SetTitle(title);
        return null;
    }

    private static object? OpenPath(string path)
    {
        if (!Directory.Exists(path) && !File.Exists(path))
            throw new FileNotFoundException($"路径不存在：{path}");
        ShellOpen(path);
        return null;
    }

    private static object? OpenUrl(string url)
    {
        if (!Uri.TryCreate(url, UriKind.Absolute, out var uri) || (uri.Scheme != Uri.UriSchemeHttps && uri.Scheme != Uri.UriSchemeHttp))
            throw new ArgumentException("只允许打开 http/https 链接");
        ShellOpen(uri.ToString());
        return null;
    }

    private static void ShellOpen(string target)
    {
        if (OperatingSystem.IsWindows())
        {
            Process.Start(new ProcessStartInfo(target) { UseShellExecute = true });
            return;
        }
        var psi = new ProcessStartInfo(OperatingSystem.IsMacOS() ? "open" : "xdg-open") { UseShellExecute = false };
        psi.ArgumentList.Add(target);
        Process.Start(psi);
    }
}
