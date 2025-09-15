-- Mixed workload test
counter = 0
max_files = 100

function weighted_choice(weights)
    local total = 0
    for _, weight in ipairs(weights) do
        total = total + weight
    end
    
    local random = math.random() * total
    local current = 0
    
    for i, weight in ipairs(weights) do
        current = current + weight
        if random <= current then
            return i
        end
    end
    return 1
end

request = function()
    counter = counter + 1
    
    local operations = {"GET", "PUT", "DELETE"}
    local weights = {50, 30, 20}
    
    local op_index = weighted_choice(weights)
    local operation = operations[op_index]
    local file_id = (counter % max_files) + 1
    local timestamp = os.time()
    local path = "/file_" .. file_id .. "_" .. timestamp .. ".dat"
    
    if operation == "GET" then
        return wrk.format("GET", path)
        
    elseif operation == "PUT" then
        local sizes = {1024, 2048, 4096}
        local size = sizes[(counter % #sizes) + 1]
        local data = string.rep("X", size)
        
        return wrk.format("PUT", path, {
            ["Content-Type"] = "application/octet-stream"
        }, data)
        
    else
        return wrk.format("DELETE", path)
    end
end