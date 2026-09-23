# A data frame and a vectorized expression.
readers <- data.frame(name = c("Ada", "Grace"), count = c(2, 3))
readers$greeting <- paste("Hello,", readers$name)
print(subset(readers, count > 2))
