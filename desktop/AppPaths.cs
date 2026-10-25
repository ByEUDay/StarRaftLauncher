namespace StarRaft.Desktop;

/// <summary>
/// 发布目录与数据目录的定位。支持两种布局：
/// <list type="bullet">
/// <item>普通目录（dist/）：可执行文件旁边就是 bin/ lib/ web/ plugins/</item>
/// <item>macOS 应用包：可执行文件在 Contents/MacOS（含 bin/），其余资源在 Contents/Resources</item>
/// </list>
/// </summary>
public sealed record AppPaths(
    string Home,
    string BackendExe,
    string? DownloadBin,
    string DataDir,
    string LogDir,
    string? DevUrl,
    bool IsBundle)
{
    public static AppPaths Resolve()
    {
        var exeDir = AppContext.BaseDirectory.TrimEnd(Path.DirectorySeparatorChar);
        var bundleResources = Path.GetFullPath(Path.Combine(exeDir, "..", "Resources"));
        var isBundle = OperatingSystem.IsMacOS()
            && Path.GetFileName(exeDir) == "MacOS"
            && Directory.Exists(Path.Combine(bundleResources, "web"));

        var home = Environment.GetEnvironmentVariable("STARRAFT_HOME") ?? (isBundle ? bundleResources : exeDir);
        var binDir = isBundle ? Path.Combine(exeDir, "bin") : Path.Combine(home, "bin");
        var exe = OperatingSystem.IsWindows() ? ".exe" : "";

        var backend = Environment.GetEnvironmentVariable("STARRAFT_BACKEND") ?? Path.Combine(binDir, "srbackend" + exe);
        // 应用包内下载引擎不在 Resources 下，需要显式告诉后端
        var download = isBundle ? Path.Combine(binDir, "srdl") : null;

        var data = Environment.GetEnvironmentVariable("STARRAFT_DATA_DIR") ?? DefaultDataDir();
        var logs = Path.Combine(data, "logs");
        Directory.CreateDirectory(logs);
        return new AppPaths(home, backend, download, data, logs, Environment.GetEnvironmentVariable("STARRAFT_DEV_URL"), isBundle);
    }

    /// <summary>与 Rust 的 dirs::data_dir() 保持一致</summary>
    private static string DefaultDataDir()
    {
        string baseDir;
        if (OperatingSystem.IsWindows())
        {
            baseDir = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
        }
        else if (OperatingSystem.IsMacOS())
        {
            baseDir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), "Library", "Application Support");
        }
        else
        {
            baseDir = Environment.GetEnvironmentVariable("XDG_DATA_HOME") is { Length: > 0 } xdg
                ? xdg
                : Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), ".local", "share");
        }
        return Path.Combine(baseDir, "StarRaftLauncher");
    }
}
