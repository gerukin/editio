# Blocks, symbols, and interpolation.
def greet(name)
  "Hello, #{name}!"
end
settings = { tab_size: 4, enabled: true }
%w[Ada Grace].each { |name| puts greet(name) }
