// FizzBuzz from 1 to 20
// Print: 0 if divisible by both 3 and 5,
//        1 if divisible by 3 only,
//        2 if divisible by 5 only,
//        the number itself otherwise.
int i = 1;
while (i <= 20) {
    if (i % 15 == 0) {
        print(0);
    } else {
        if (i % 3 == 0) {
            print(1);
        } else {
            if (i % 5 == 0) {
                print(2);
            } else {
                print(i);
            }
        }
    }
    i = i + 1;
}
