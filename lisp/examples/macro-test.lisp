(def! inc (fn* [a] (i+ a 1)))
(def! gensym
  (let* [counter (atom 0)]
    (fn* []
      (symbol (str "G__" (swap! counter inc))))))

(defmacro! zk-square (fn* [var] (
        (let* [v1 (gensym)
               v2 (gensym)] (
        `(alloc ~v1 ~var)
        `(def! output (alloc ~v2 (square ~var)))
        `(enforce  
            (scalar::one ~v1) 
            (scalar::one ~v1) 
            (scalar::one ~v2) 
         )
        `{ "v2" output }
        )
    ))
))

(defmacro! zk-mul (fn* [val1 val2] (
        (let* [v1 (gensym)
               v2 (gensym)
               var (gensym)] (
        `(alloc ~v1 ~val1)
        `(alloc ~v2 ~val2)
        `(def! result (alloc ~var (* ~val1 ~val2)))
        `(enforce  
            (scalar::one ~v1) 
            (scalar::one ~v2) 
            (scalar::one ~var) 
         )
        `{ "result" result }
        )
    ))
))

;; -u^2 + v^2 = 1 + du^2v^2
(defmacro! zk-witness (fn* [val1 val2] (
        (let* [u2v2 (gensym)] (
        `(def! ~EDWARDS_D (alloc-const ~EDWARDS_D (scalar "2a9318e74bfa2b48f5fd9207e6bd7fd4292d7f6d37579d2601065fd6d6343eb1")))
        `(def! u2 (alloc ~u2 (get (nth (nth (zk-square ~val1) 0) 3) "v2")))
        `(def! v2 (alloc ~v2 (get (nth (nth (zk-square ~val2) 0) 3) "v2")))
        `(alloc ~u2v2 (get (last (last (zk-mul u2 v2))) "result"))
        `(enforce  
            ((scalar::one::neg ~u2) (scalar::one ~v2))
            (scalar::one cs::one) 
            ((scalar::one cs::one) (~EDWARDS_D ~u2v2))
        )
        )
    ))
))


(def! param1 (scalar 3))
(def! param2 (scalar 1))
(prove 
  (
    ;; (def! result1 (zk-square param1))
    ;; (println 'result1_map (get (nth (nth result1 0) 3) "v2"))
    ;; (def! result2 (zk-square param2))
    ;; (println 'result_mul (get (last (last (zk-mul param1 param1))) "result"))
  )
)