validate_inputs <- function(density, domain, n, points) {
 if (!is.function(density) || !is.numeric(domain) || length(domain)!=2 || anyNA(domain) || domain[1]>=domain[2]) stop("domain")
 if (!is.numeric(n) || length(n)!=1 || !is.finite(n) || n<1 || n!=floor(n)) stop("count")
 if (!is.numeric(points) || length(points)<3 || any(!is.finite(points)) || any(diff(points)<=0) || any(points<=domain[1] | points>=domain[2])) stop("points")
 y <- density(points)
 if (!is.numeric(y) || length(y)!=length(points) || any(!is.finite(y)) || any(y<=0)) stop("density")
 if (any(diff(diff(log(y))/diff(points))>1e-9)) stop("not log concave")
 TRUE
}
