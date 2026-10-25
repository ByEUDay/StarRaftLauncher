package engine

import "strings"

// BMCLAPI 镜像映射（https://bmclapidoc.bangbang93.com）
var bmclapiRules = []struct{ from, to string }{
	{"https://piston-meta.mojang.com", "https://bmclapi2.bangbang93.com"},
	{"https://piston-data.mojang.com", "https://bmclapi2.bangbang93.com"},
	{"https://launchermeta.mojang.com", "https://bmclapi2.bangbang93.com"},
	{"https://launcher.mojang.com", "https://bmclapi2.bangbang93.com"},
	{"https://resources.download.minecraft.net", "https://bmclapi2.bangbang93.com/assets"},
	{"https://libraries.minecraft.net", "https://bmclapi2.bangbang93.com/maven"},
	{"https://maven.fabricmc.net", "https://bmclapi2.bangbang93.com/maven"},
	{"https://maven.minecraftforge.net", "https://bmclapi2.bangbang93.com/maven"},
	{"https://files.minecraftforge.net/maven", "https://bmclapi2.bangbang93.com/maven"},
	{"https://maven.neoforged.net/releases", "https://bmclapi2.bangbang93.com/maven"},
}

// Candidates 返回按优先级排列的下载地址：镜像在前，官方源兜底。
func Candidates(url, mirror string) []string {
	if mirror == "bmclapi" {
		for _, r := range bmclapiRules {
			if strings.HasPrefix(url, r.from) {
				return []string{r.to + strings.TrimPrefix(url, r.from), url}
			}
		}
	}
	return []string{url}
}
