-- GET large files test
counter = 0
max_files = 100

request = function()
    counter = counter + 1
    local file_id = (counter % max_files) + 1
    local path = "/large_file_" .. file_id .. ".dat"
    
    return wrk.format("GET", path)
end