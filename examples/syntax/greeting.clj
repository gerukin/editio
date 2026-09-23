;; Transform a collection.
(ns demo.greeting)
(defn greet [name]
  (str "Hello, " name "!"))
(map greet ["Ada" "Grace"])
