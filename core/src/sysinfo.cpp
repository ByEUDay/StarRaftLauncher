#include "starraft/core.h"

#if defined(_WIN32)
#  define WIN32_LEAN_AND_MEAN
#  include <windows.h>
#elif defined(__APPLE__)
#  include <mach/mach.h>
#  include <sys/sysctl.h>
#else
#  include <fstream>
#  include <sstream>
#  include <string>
#endif

extern "C" int sr_system_memory(uint64_t* total, uint64_t* available) {
    if (!total || !available) return -1;
    *total = *available = 0;

#if defined(_WIN32)
    MEMORYSTATUSEX st;
    st.dwLength = sizeof(st);
    if (!GlobalMemoryStatusEx(&st)) return -2;
    *total = st.ullTotalPhys;
    *available = st.ullAvailPhys;
    return 0;

#elif defined(__APPLE__)
    uint64_t mem = 0;
    size_t size = sizeof(mem);
    if (sysctlbyname("hw.memsize", &mem, &size, nullptr, 0) != 0) return -2;
    *total = mem;

    vm_size_t page = 0;
    vm_statistics64_data_t vm;
    mach_msg_type_number_t count = HOST_VM_INFO64_COUNT;
    const mach_port_t host = mach_host_self();
    if (host_page_size(host, &page) == KERN_SUCCESS &&
        host_statistics64(host, HOST_VM_INFO64, reinterpret_cast<host_info64_t>(&vm), &count) == KERN_SUCCESS) {
        *available = (static_cast<uint64_t>(vm.free_count) + vm.inactive_count + vm.purgeable_count) * page;
    }
    return 0;

#else
    std::ifstream in("/proc/meminfo");
    if (!in) return -2;
    std::string line;
    while (std::getline(in, line)) {
        std::istringstream ss(line);
        std::string key;
        uint64_t kb = 0;
        ss >> key >> kb;
        if (key == "MemTotal:") *total = kb * 1024;
        else if (key == "MemAvailable:") *available = kb * 1024;
    }
    return *total ? 0 : -3;
#endif
}
