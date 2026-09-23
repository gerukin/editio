% Vectors and element-wise arithmetic.
x = [1, 2, 3, 4];
y = x .^ 2;
for index = 1:length(y)
    fprintf('Value: %d\n', y(index));
end
