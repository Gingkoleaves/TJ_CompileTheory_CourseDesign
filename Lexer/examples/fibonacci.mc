// Print the first 10 Fibonacci numbers (0-indexed)
int n = 10;
int a = 0;
int b = 1;
int i = 0;

while (i < n) {
    print(a);
    int tmp = a + b;
    a = b;
    b = tmp;
    i = i + 1;
}
