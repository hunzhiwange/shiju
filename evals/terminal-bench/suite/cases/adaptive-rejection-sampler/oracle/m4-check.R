source(file.path(candidate,"answer.R"))
# Empirical CDF, moments, support, size, and RNG reproducibility on synthetic bounded targets.
probe <- function(f,d,n,ceiling,cdf,mean0,sd0) {
 set.seed(731); x<-ars(f,d,n=n,ceiling=ceiling)
 stopifnot(is.numeric(x),length(x)==n,all(is.finite(x)),all(x>=d[1] & x<=d[2]),length(unique(x))>n*.9)
 set.seed(731); y<-ars(f,d,n=n,ceiling=ceiling); stopifnot(identical(x,y))
 stopifnot(abs(mean(x)-mean0)<.04*(d[2]-d[1]),abs(sd(x)-sd0)<.04*(d[2]-d[1]))
 sx<-sort(x); e<-cdf(sx)
 stopifnot(max(abs(e-seq_len(n)/n),abs(e-(seq_len(n)-1)/n))<.035)
}
probe(function(x) x,c(0,1),4000,1,function(x) x*x,2/3,sqrt(1/18))
probe(function(x) exp(-x),c(0,3),4000,1,function(x) (1-exp(-x))/(1-exp(-3)),1-3/(exp(3)-1),sqrt(1-9*exp(3)/(exp(3)-1)^2))
probe(function(x) 5*(x-2),c(2,4),4000,10,function(x) ((x-2)/2)^2,2+4/3,2*sqrt(1/18))
stopifnot(inherits(try(ars(function(x) 1,c(1,0),3,1),silent=TRUE),"try-error"))
