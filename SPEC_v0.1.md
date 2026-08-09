# 📖 مواصفات لغة `fast_lang` - الإصدار v0.1 (Language Specification)

هذه الوثيقة تمثل المرجع الأساسي والرسمي لقواعد النحو (Syntax) الخاصة بالإصدار الأولي (v0.1) من مترجم `fast_lang`.

---

## 1. الأنواع الأولية (Primitive Types)
اللغة تدعم أنواعاً صارمة مع تحديد حجم الذاكرة بدقة (مثل Rust و C++).
*ملاحظة: سيتم إضافة أنواع أخرى مستقبلاً مثل `uint`, `usize`, `byte`.*

```fast
int(8)    // 8-bit integer
int(32)   // 32-bit integer
int(64)   // 64-bit integer
float(32) // 32-bit float
float(64) // 64-bit float
bool      // true / false
char      // Single character
str       // String (Not a linked list!)
array(T)  // Generic Array of type T (Contiguous memory, Not a linked list!)
```

---

## 2. تعريف المتغيرات (Variable Declarations)
المتغيرات تُعرّف باستخدام `let` للمتغيرات القابلة للتعديل، و `const` للثوابت.

```fast
let int(32) x -> 10;
const float(64) pi -> 3.14;
let array(int(32)) my_arr -> [1, 2, 3];
```

---

## 3. الـ Scope: الكيان الجوهري للغة (The Omni-Scope)
الـ Scope هو المكون المعماري الأساسي. يمكن للـ Scope أن يتخذ عدة أشكال بناءً على الخاصية `type ->`. 

### أ. الهيكل العام للـ Scope المخصص (`custom`)
```fast
scope MyList -> {
    type -> custom;
    
    // إعدادات الـ Scope (Settings)
    enable [index_access, custom_init_body, length];
    
    // فلاجات مسار التنفيذ (Control Flow Flags)
    enable flag[is_break];
    
    // إضافة حقول ديناميكية (Dynamic Fields)
    add int(32) size;
    add int(32) length;

    // دالة البناء (Constructor)
    _(array(int(32)) arr) -> {
        set this.size = arr.length;
    }

    // الأعضاء العامة والخاصة (Access Modifiers)
    public -> {
        fn get_size() -> int(32) {
            return this.size;
        }
    }
}
```

---

## 4. الدوال (Functions)
الـ `fn` هي مجرد (Syntactic Sugar) لـ `scope` من نوع `Fn`. يمكن كتابتها بالشكل التقليدي:

```fast
fn add_numbers(a: int(32), b: int(32)) -> int(32) {
    return a + b;
}
```

---

## 5. الهياكل والفئات التقليدية (Structs & Classes)
كما هو الحال مع الدوال، الـ `struct` والـ `class` هي قوالب مسبقة التجهيز.

```fast
struct Point {
    public -> {
        int(32) x;
        int(32) y;
    }
}
```

---

## 6. الجمل الشرطية وحلقات التكرار (Control Flow)

### الجمل الشرطية (If / Else)
```fast
if (x > 10) {
    log("Greater");
} else {
    log("Smaller");
}
```

### حلقات التكرار (Loops)
```fast
while (x > 0) {
    x = x - 1;
}

loop {
    if (x == 5) {
        break;
    }
}
```

---

## 7. معالجة الأخطاء (Error Handling)
تم بناء معالجة الأخطاء كجزء أساسي من اللغة. يمكنك استخدام `try` و `catch` لالتقاط الأخطاء، و `throw` لإرسالها. 
كما سيتم دمجها مع قوة הـ `handle` للتعامل مع الأحداث بشكل ديناميكي.

```fast
try {
    throw new error("Something went wrong");
} catch (e: error) {
    log(e.message);
}

// يمكن أيضاً اعتراض الأخطاء باستخدام الـ Handle
handle.is_error -> {
    log("An error occurred");
}
```

---

## 8. تعيين القيم (Assignment)
اللغة تستخدم `->` للتعيين:
```fast
set this.size -> 10;
```

---
*هذه الوثيقة سيتم تحديثها باستمرار مع تقدم بناء المترجم لتعكس القواعد النحوية المدعومة بدقة 100%.*
