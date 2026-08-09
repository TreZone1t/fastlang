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


namespace std_list {
template <typename T>
struct Node {
    Node(T value) {
        this->value = value;
        this->next = nullptr;
    }
    public:
    void set_next(void* node) {
        this->next = node;
    }
    void* get_next() {
        return this->next;
    }
    void set_value(T value) {
        this->value = value;
    }
    T get_value() {
        return this->value;
    }
private:
    T value;
    void* next = nullptr;
};
template <typename T>
struct LinkedList {
    LinkedList(std::vector<T> arr) {
        this->extend_from_array(arr);
    }
    void extend_from_array(std::vector<T> arr) {
        {
            int32_t i = 0;
            while ((i < arr.size())) {
                this->push(arr[i]);
                i++;
            }
        }
    }
    public:
    void set_head(void* node) {
        this->head = node;
    }
    void* get_head() {
        return this->head;
    }
    void push(T item) {
        if ((this->head == nullptr)) {
            this->head = (void*)new Node<T>(item);
        } else {
            void* new_node = (void*)new Node<T>(item);
            ((std_list::Node<T>*)new_node)->set_next(this->head);
            this->head = new_node;
        }
    }
    std::optional<T> pop() {
        void* temp = this->head;
        if ((temp != nullptr)) {
            this->head = ((std_list::Node<T>*)temp)->get_next();
            T val = ((std_list::Node<T>*)temp)->get_value();
            delete temp;
            return std::optional{val};
        } else {
            return std::nullopt;
        }
    }
private:
    void* head = nullptr;
};


} // namespace std_list

using namespace std_list;
int main() {
    std_list::LinkedList<int32_t> li = std::vector{1, 2, 3, 4, 5, 6};
    li.push(7);
    std::optional<int> val1 = li.pop();
    std::optional<int> val2 = li.pop();
    std::optional<int> val3 = li.pop();
    std::optional<int> val4 = li.pop();
    std::optional<int> val5 = li.pop();
    std::optional<int> val6 = li.pop();
    std::optional<int> val7 = li.pop();
    std::optional<int> val8 = li.pop();
}

