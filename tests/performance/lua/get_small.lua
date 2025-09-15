-- GET small files test
counter = 0
max_files = 1000

request = function()
    counter = counter + 1
    local file_id = (counter % max_files) + 1
    local path = "/small_file_" .. file_id .. ".dat"
    
    return wrk.format("GET", path)
end