#include <iostream>
#include <string>
#include <vector>
#include <memory>
#include <stdexcept>
#include <cstdint>

using namespace std;

std::ostream& operator<<(std::ostream& os, const std::exception& e) {
    return os << e.what();
}


void Box(int32_t width, int32_t height) {
}

int main() {
    return 0;
}
