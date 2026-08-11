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


struct None {
    public:
    bool is_none() {
        return true;
    }
    bool is_some() {
        return false;
    }
};
struct Some {
    Some(T value) {
        this->value = value;
    }
    public:
    void set_value(T value) {
        this->value = value;
    }
    T get_value() {
        return this->value;
    }
    bool is_some() {
        return true;
    }
    bool is_none() {
        return false;
    }
private:
    T value;
};
struct Option {
    Option() {
        this->is_some = false;
    }
    public:
    void set_value(T value) {
        this->value = value;
        this->is_some = true;
    }
    T get_value() {
        return this->value;
    }
    void set_is_some(bool is_some) {
        this->is_some = is_some;
    }
    bool get_is_some() {
        return this->is_some;
    }
private:
    T value;
    bool is_some;
};

int main() {
    return 0;
}
