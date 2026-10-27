ars <- function(density_function, domain, n, ceiling) {
 if (!is.function(density_function) || length(domain)!=2 || any(!is.finite(domain)) || domain[1]>=domain[2] || length(n)!=1 || !is.finite(n) || n<1 || n!=floor(n) || !is.finite(ceiling) || ceiling<=0) stop("inputs")
 out <- numeric(n); i <- 0L; attempts <- 0L
 while (i<n) {
   attempts <- attempts+1L
   if (attempts>1000000) stop("proposal limit")
   x <- runif(1,domain[1],domain[2]); y <- density_function(x)
   if (length(y)!=1 || !is.finite(y) || y<0 || y>ceiling*(1+1e-12)) stop("density bound")
   if (runif(1)*ceiling<=y) { i<-i+1L; out[i]<-x }
 }
 out
}
