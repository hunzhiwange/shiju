source(file.path(candidate,"answer.R"))
close <- function(a,b) stopifnot(is.numeric(a),length(a)==length(b),all(is.finite(a)),max(abs(a-b))<1e-9)
for (a in c(.4,1.3)) for (mu in c(-.7,1.1)) {
 x<-c(-2,-.3,1.9,3); h<- -a*(x-mu)^2+2.7; dh<- -2*a*(x-mu)
 ans<-hull(x,h,dh,c(-4,5))
 close(ans$slopes,dh); close(ans$intercepts,a*x*x-a*mu*mu+2.7)
 close(ans$z,c(-4,(x[-1]+x[-length(x)])/2,5))
 close(ans$lower_slopes,-a*(x[-1]+x[-length(x)]-2*mu))
 close(ans$lower_intercepts,a*x[-1]*x[-length(x)]-a*mu*mu+2.7)
 for(t in seq(-3.9,4.9,length.out=31)) {
  j<-findInterval(t,ans$z); stopifnot(ans$slopes[j]*t+ans$intercepts[j]>= -a*(t-mu)^2+2.7-1e-9)
 }
}
# Asymmetric, non-quadratic concave log density h=log(x), tangent intersections.
x<-c(.3,1.4,3.8); h<-log(x); dh<-1/x
ans<-hull(x,h,dh,c(.1,6)); b<-log(x)-1
close(ans$z,c(.1,(b[2]-b[1])/(dh[1]-dh[2]),(b[3]-b[2])/(dh[2]-dh[3]),6))
close(ans$lower_slopes,diff(log(x))/diff(x))
