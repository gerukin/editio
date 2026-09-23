;; A function and a list.
(defun greet (name)
  (format nil "Hello, ~A!" name))
(mapcar #'greet '("Ada" "Grace"))
