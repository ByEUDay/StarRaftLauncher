using System.Net;
using Photino.NET;

namespace StarRaft.Desktop;

/// <summary>
/// GUI 桌面宿主：创建原生窗口，托管 BackEnd 进程，加载 FrontEnd，提供原生能力桥。
/// </summary>
internal static class Program
{
    [STAThread]
    private static void Main(string[] args)
    {
        var paths = AppPaths.Resolve();
        var devMode = paths.DevUrl is not null || args.Contains("--devtools");

        var window = new PhotinoWindow()
            .SetTitle("StarRaft Launcher")
            .SetUseOsDefaultSize(false)
            .SetSize(1200, 780)
            .SetMinSize(960, 620)
            .Center()
            .SetResizable(true)
            .SetDevToolsEnabled(devMode)
            .SetContextMenuEnabled(devMode);

        // macOS 的程序坞图标来自应用包的 AppIcon.icns，其它平台设置窗口图标
        if (!OperatingSystem.IsMacOS())
        {
            var iconPath = Path.Combine(AppContext.BaseDirectory, "Assets", OperatingSystem.IsWindows() ? "icon.ico" : "icon.png");
            if (File.Exists(iconPath)) window.SetIconFile(iconPath);
        }

        var bridge = new NativeBridge(window, paths);
        window.RegisterWebMessageReceivedHandler(bridge.OnMessage);

        using var backend = new BackendHost(paths);
        try
        {
            window.Load(new Uri(backend.Start(TimeSpan.FromSeconds(30))));
        }
        catch (Exception e)
        {
            Console.Error.WriteLine(e);
            window.LoadRawString(ErrorPage(e, paths));
        }

        window.WaitForClose();
    }

    private static string ErrorPage(Exception e, AppPaths paths) => $$"""
        <!doctype html>
        <html lang="zh-CN"><head><meta charset="utf-8"><title>StarRaft Launcher</title>
        <style>
          body { font-family: system-ui, sans-serif; background:#0f1220; color:#e6e8f2; padding:48px; }
          h1 { font-size:22px; } code, pre { background:#1a1f35; padding:12px; border-radius:8px; display:block; white-space:pre-wrap; }
        </style></head>
        <body>
          <h1>启动器后端无法启动</h1>
          <pre>{{WebUtility.HtmlEncode(e.Message)}}</pre>
          <p>日志目录：</p><code>{{WebUtility.HtmlEncode(paths.LogDir)}}</code>
        </body></html>
        """;
}
