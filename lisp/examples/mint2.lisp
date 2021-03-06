(load-file "util.lisp")

(def! zk-not-small-order? (fn* [u v] (
        (def! first-doubling (last (last (zk-double u v))))
        (def! second-doubling (last (last 
            (zk-double (get first-doubling "u3") (get first-doubling "v3")))))
        (def! third-doubling (last (last 
            (zk-double (get second-doubling "u3") (get second-doubling "v3")))))
        (zk-nonzero? (get third-doubling "u3"))
        )
    )
)

(defmacro! zk-nonzero? (fn* [var] (
        (let* [inv (gensym)
               v1 (gensym)] (
        `(alloc ~inv (invert ~var))
        `(alloc ~v1 ~var)
        `(enforce  
            (scalar::one ~v1) 
            (scalar::one ~inv) 
            (scalar::one cs::one) 
         )
        )
    ))
))

(defmacro! zk-square (fn* [var] (
        (let* [v1 (gensym)
               v2 (gensym)] (
        `(alloc ~v1 ~var)
        `(def! output (alloc-input ~v2 (square ~var)))
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
        `(def! result (alloc-input ~var (* ~val1 ~val2)))
        `(enforce  
            (scalar::one ~v1) 
            (scalar::one ~v2) 
            (scalar::one ~var) 
         )
        `{ "result" result }
        )
    ))
))

(defmacro! zk-witness (fn* [val1 val2] (
        (let* [u2 (gensym)
               v2 (gensym)
               u2v2 (gensym)
               EDWARDS_D (gensym)] (
        `(def! ~EDWARDS_D (alloc-const ~EDWARDS_D (scalar "2a9318e74bfa2b48f5fd9207e6bd7fd4292d7f6d37579d2601065fd6d6343eb1")))
        `(def! ~u2 (alloc ~u2 (get (nth (nth (zk-square ~val1) 0) 3) "v2")))
        `(def! ~v2 (alloc ~v2 (get (nth (nth (zk-square ~val2) 0) 3) "v2")))
        `(def! result (alloc-input ~u2v2 (get (last (last (zk-mul ~u2 ~v2))) "result")))        
        `(enforce  
            ((scalar::one::neg ~u2) (scalar::one ~v2))
            (scalar::one cs::one)
            ((scalar::one cs::one) (~EDWARDS_D ~u2v2))
         )
        `{ "result" result }
        )
    ))
))

(defmacro! zk-double (fn* [val1 val2] (
        (let* [u (gensym)
               v (gensym)
               u3 (gensym)
               v3 (gensym)
               T (gensym)
               A (gensym)
               C (gensym)
               EDWARDS_D (gensym)] (
        `(def! ~EDWARDS_D (alloc-const ~EDWARDS_D (scalar "2a9318e74bfa2b48f5fd9207e6bd7fd4292d7f6d37579d2601065fd6d6343eb1")))
        `(def! ~u (alloc ~u ~val1))
        `(def! ~v (alloc ~v ~val2))
        `(def! ~T (alloc ~T (* (+ ~val1 ~val2) (+ ~val1 ~val2))))
        `(def! ~A (alloc ~A (* ~u ~v)))
        `(def! ~C (alloc ~C (* (square ~A) ~EDWARDS_D)))
        `(def! ~u3 (alloc ~u3 (/ (double ~A) (+ scalar::one ~C))))
        `(def! ~v3 (alloc ~v3 (/ (- ~T (double ~A)) (- scalar::one ~C))))
        `(enforce  
            ((scalar::one ~u) (scalar::one ~v))
            ((scalar::one ~u) (scalar::one ~v))
            (scalar::one ~T)
         )
         `(enforce  
            (~EDWARDS_D ~A)
            (scalar::one ~A)
            (scalar::one ~C)
         )
         `(enforce  
            ((scalar::one cs::one) (scalar::one ~C))
            (scalar::one ~u3)
            ((scalar::one ~A) (scalar::one ~A))    
         )
         `(enforce  
            ((scalar::one cs::one) (scalar::one::neg ~C))
            (scalar::one ~v3)
            ((scalar::one ~T) (scalar::one::neg ~A) (scalar::one::neg ~A))    
         )    
        { "u3" u3, "v3" v3 }
        )
    ))
))

(defmacro! conditionally-select (fn* [val1 val2 val3] (
        (let* [u-prime (gensym)
               v-prime (gensym)
               u (gensym)
               v (gensym)
               condition (gensym)
               ] (
            `(def! ~u (alloc ~u ~val1))
            `(def! ~v (alloc ~v ~val2))
            `(def! ~condition (alloc ~condition ~val3))
            `(def! ~u-prime (alloc ~u-prime (* ~u ~condition)))
            `(def! ~v-prime (alloc ~v-prime (* ~v ~condition)))
            `(enforce
                (scalar::one ~u)
                (scalar::one ~condition)
                (scalar::one ~u-prime)
             )
            `(enforce
                (scalar::one ~v)
                (scalar::one ~condition)
                (scalar::one ~v-prime)
             )
             { "u-prime" u-prime, "v-prime" v-prime }
        )
))))

(defmacro! jj-add (fn* [param1 param2 param3 param4]
    (let* [u1 (gensym) v1 (gensym) u2 (gensym) v2 (gensym)
           EDWARDS_D (gensym) U (gensym) A (gensym) B (gensym)
           C (gensym) u3 (gensym) v3 (gensym)] (
        ;; debug
        ;; `(println 'jj-add ~param1 ~param2 ~param3 ~param4)
        `(def! ~u1 (alloc ~u1 ~param1))
        `(def! ~v1 (alloc ~v1 ~param2))
        `(def! ~u2 (alloc ~u2 ~param3))
        `(def! ~v2 (alloc ~v2 ~param4)) 
        `(def! ~EDWARDS_D (alloc-const ~EDWARDS_D (scalar "2a9318e74bfa2b48f5fd9207e6bd7fd4292d7f6d37579d2601065fd6d6343eb1")))
        `(def! ~U (alloc ~U (* (+ ~u1 ~v1) (+ ~u2 ~v2))))
        `(def! ~A (alloc ~A (* ~v2 ~u1)))
        `(def! ~B (alloc ~B (* ~u2 ~v1)))
        `(def! ~C (alloc ~C (* ~EDWARDS_D (* ~A ~B))))
        `(def! ~u3 (alloc ~u3 (/ (+ ~A ~B) (+ scalar::one ~C))))
        `(def! ~v3 (alloc ~v3 (/ (- (- ~U ~A) ~B) (- scalar::one ~C))))        
  `(enforce  
    ((scalar::one ~u1) (scalar::one ~v1))
    ((scalar::one ~u2) (scalar::one ~v2))
    (scalar::one ~U)
   )
  `(enforce
    (~EDWARDS_D ~A)
    (scalar::one ~B)
    (scalar::one ~C)
   )
  `(enforce
    ((scalar::one cs::one)(scalar::one ~C))
    (scalar::one ~u3)
    ((scalar::one ~A) (scalar::one ~B))
   )
  `(enforce
    ((scalar::one cs::one) (scalar::one::neg ~C))
    (scalar::one ~v3)
    ((scalar::one ~U) (scalar::one::neg ~A) (scalar::one::neg ~B))
   ) 
   { "u3" u3, "v3" v3 }
  )  
)
))

(defmacro! zk-boolean (fn* [val] (
        (let* [var (gensym)] (
            `(alloc ~var ~val)
            `(enforce
                ((scalar::one cs::one) (scalar::one::neg ~var))
                (scalar::one ~var)
                ()
             )
        )
))))

(def! jj-mul (fn* [u v b] (
    (def! result (unpack-bits b))
    (eval (map zk-boolean result))
    (def! val (last (last (zk-double u v))))
    (def! acc 0)
    (dotimes (count result) (                    
        (def! u3 (get val "u3"))
        (def! v3 (get val "v3"))            
        (def! r (nth result acc))        
        (def! cond-result (last (last (conditionally-select u3 v3 r))))
        (def! u-prime (get cond-result "u-prime"))
        (def! v-prime (get cond-result "v-prime"))        
        (def! add-result (last (jj-add u3 v3 u-prime v-prime)))               
        (def! u-add (get add-result "u3"))
        (def! v-add (get add-result "v3"))        
        (def! val (last (last (zk-double u-add v-add))))             
        (println acc val)    
        (def! acc (i+ acc 1))        
    ))
    (val)
    ;; { "u3" (get val "u3"), "v3" (get val "v3") }    
)))

(load-file "mimc-constants.lisp")
(defmacro! mimc-macro (fn* [left-value right-value acc] (
    (let* [tmp-xl (gensym2 'tmp_xl) 
        xl-new-value (gensym2 'xl_new_value) 
        cur-mimc-const (gensym2 'cur_mimc_const)
        xl (gensym2 'xl) 
        xr (gensym2 'xr)] (
    `(def! ~xl (alloc ~xl ~left-value))
    `(def! ~xr (alloc ~xr ~right-value))
    `(def! ~cur-mimc-const (alloc-const ~cur-mimc-const (nth mimc-constants ~acc)))
    `(def! ~tmp-xl (alloc ~tmp-xl (square (+ ~cur-mimc-const ~xl))))        
    `(enforce 
        ((scalar::one ~xl) (~cur-mimc-const cs::one))
        ((scalar::one ~xl) (~cur-mimc-const cs::one))
        (scalar::one ~tmp-xl)
    )   
    `(def! new-value (+ (* ~tmp-xl (+ ~cur-mimc-const ~xl)) ~xr))
    `(if (= ~acc 321)        
        (def! ~xl-new-value (alloc-input ~xl-new-value new-value))
        (def! ~xl-new-value (alloc ~xl-new-value new-value))
    )
    `(enforce 
        (scalar::one ~tmp-xl)
        ((scalar::one ~xl) (~cur-mimc-const cs::one))            
        ((scalar::one ~xl-new-value) (scalar::one::neg ~xr))            
    )
    `{ "left" new-value }    
    )    
))))

(def! mimc (fn* [left right] (
    (def! acc 0)
    (def! xl left)
    (def! xr right)
    (dotimes 322 (        
        (def! result (mimc-macro xl xr acc))
        (def! result-value (get (last (last result)) "left"))
        (def! xr xl)
        (def! xl result-value)
        (def! acc (i+ acc 1))        
    ))
    { "result" result-value }
)))

(defmacro! rangeproof-alloc (fn* [value value-digit] (
 (let* [bit (gensym2 'bit)
        digit (gensym2 'digit)] (
    `(def! ~bit (alloc ~bit ~value))
    `(def! ~digit (alloc-const ~digit ~value-digit))
    `(enforce 
        (scalar::one ~bit) 
        ((scalar::one cs::one) (scalar::one::neg ~bit))
        () 
    )    
    { "lc" ((str digit) (str bit)) }
)))))

(def! rangeproof (fn* [value] (    
    (def! values-bit (unpack-bits value))
    (def! idx 0)
    (def! digit (scalar 1))
    (def! value-result ())
    (dotimes 64 (
        (def! bit (nth values-bit idx))    
        (def! value-result 
            (conj value-result 
            (get (last (last 
                (rangeproof-alloc bit digit))) "lc")))
        (def! digit (double digit))
        (def! idx (i+ idx 1))
    ))
    (def! value-alloc (alloc-input "value-alloc" value))
    (enforce 
        (value-result)
        (scalar::one cs::one)
        (scalar::one value-alloc)
    )  
)))

;; mint contract
(def! generator-coin-u (scalar "0d7b70a0c82cbabf8f59ee61a63b8e0adcff42e9f2da7bda84f9308b3531dd18"))
(def! generator-coin-v (scalar "0d89cafb242b9e892153ac70335956e6f5c042997da77cf5e164233a9bbfb7b4"))
(def! generator-value-commit-u (scalar "01ae4ea270f5c6a1c0cd1dd4e067a82110fa27409dfc0aa4edd18883897a4c6b"))
(def! generator-value-commit-v (scalar "09d2a25018194750e9adacf78531ee3bfddbadd767671d517aa788c352641ff1"))
(def! generator-value-random-u (scalar "002924d15ccf8014ce724a41753d17dce3a9f7382a3db18fba3c8e286bb77382"))
(def! generator-value-random-v (scalar "0cb825b790b0601c4999e52d9added7d10d013b33fd95ca7d2ddd51691a09075"))
(def! mint-contract (fn* [public-u public-v value serial rnd-coin rnd-value] (
    (def! mimc-round-1 (get (last (mimc public-u public-v)) "result"))
    (def! mimc-round-2 (get (last (mimc mimc-round-1 value)) "result"))
    (def! mimc-round-3 (get (last (mimc mimc-round-2 serial)) "result"))
    (def! coin (get (last (mimc mimc-round-3 rnd-coin)) "result"))
    (rangeproof value)
    (def! result-mul-value 
        (last (last (jj-mul generator-value-commit-u generator-value-commit-v value))))
    (def! result-mul-rnd-value 
        (last (last (jj-mul generator-value-random-u generator-value-random-v rnd-value))))
    (def! add-result (jj-add (get result-mul-value "u3") (get result-mul-value "v3") 
            (get result-mul-rnd-value "u3") (get result-mul-rnd-value "v3")))
    (println 'add-result add-result)
    ;;(alloc-input "value-commit" add-result)
)))

;; (def! spend-contract (fn* 
;;     [secret-u secret-v serial coin-merkle-branch coin-merkle-is-right] (
;; (def! nullifier (mimc secret serial))

;; )))

(prove 
  (            
    (def! public-u (scalar "0d7b70a0c82cbabf8f59ee61a63b8e0adcff42e9f2da7bda84f9308b3531dd18"))
    (def! public-v (scalar "0cb825b790b0601c4999e52d9added7d10d013b33fd95ca7d2ddd51691a09075"))
    (def! value (scalar 3))
    (def! serial (scalar 4))
    (def! rnd-coin (rnd-scalar))
    (def! rnd-value (rnd-scalar))
    (mint-contract public-u public-v value serial rnd-coin rnd-value)
  )
)
