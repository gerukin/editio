-- Tables, functions, and iteration.
local names = {"Ada", "Grace"}
local function greet(name)
  return "Hello, " .. name
end
for _, name in ipairs(names) do print(greet(name)) end
