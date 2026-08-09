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


int32_t global_counter = 0;
const double pi = 3.14;
int main() {
    bool is_active = true;
    char initial = 'A';
    str<255> message = "Hello";
    std::array<int32_t, 3> numbers = std::vector{1, 2, 3};
    return 0;
}

