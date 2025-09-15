-- PUT large files test
counter = 0
sizes = {102400, 512000, 1048576} -- 100KB, 500KB, 1MB

request = function()
    counter = counter + 1
    local size = sizes[(counter % #sizes) + 1]
    local data = string.rep("B", size)
    local timestamp = os.time()
    local path = "/large_file_" .. counter .. "_" .. timestamp .. ".dat"
    
    return wrk.format("PUT", path, {
        ["Content-Type"] = "application/octet-stream"
    }, data)
end