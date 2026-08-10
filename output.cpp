#include <iostream>
#include <vector>
#include <array>
#include <memory>
#include <optional>
#include <stdexcept>

using namespace std;

std::ostream& operator<<(std::ostream& os, const std::exception& e) {
    return os << e.what();
}


int main() {
    std::array<int32_t, 3> arr1 = {1, 2, 3};
    int32_t y = 10;
    auto test_magic = [&]() {
        auto x = (&y);
        std::cout << (*x) << std::endl;
        std::array<int32_t, 3> arr2 = arr1;
        struct Node {
            int32_t value;
            Node(int32_t value) {
                this->value = value;
            }
        };
        int32_t temp = 10;
        Node n = Node(temp);
    };
    test_magic();
}

