decide <- function(logu, lower, upper, log_density) {
 if (length(logu)!=1 || !is.finite(logu) || logu>0 || !is.finite(upper) || is.na(lower) || lower>upper) stop("bounds")
 if (logu<=lower-upper) return(c(accept=TRUE,update=FALSE))
 h <- log_density()
 if (length(h)!=1 || !is.finite(h) || h>upper+1e-10 || h<lower-1e-10) stop("invalid envelope")
 c(accept=exp(logu)<=exp(h-upper),update=TRUE)
}
