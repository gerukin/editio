module Main where

-- Pure functions and a list comprehension.
greet :: String -> String
greet name = "Hello, " ++ name

main :: IO ()
main = mapM_ putStrLn [greet n | n <- ["Ada", "Grace"]]
