hull <- function(x, h, dh, domain) {
 b <- h-dh*x
 z <- x[-1]
 lm <- diff(h)/diff(x)
 list(slopes=dh, intercepts=b, z=c(domain[1],z,domain[2]),
      lower_slopes=lm, lower_intercepts=h[-length(h)]-lm*x[-length(x)])
}
