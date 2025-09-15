-- PUT small files test
counter = 0
sizes = {1024, 2048, 4096} -- 1KB, 2KB, 4KB

request = function()
    counter = counter + 1
    local size = sizes[(counter % #sizes) + 1]
    local data = string.rep("A", size)
    local timestamp = os.time()
    local path = "/small_file_" .. counter .. "_" .. timestamp .. ".dat"
    
    return wrk.format("PUT", path, {
        ["Content-Type"] = "application/octet-stream"
    }, data)
end