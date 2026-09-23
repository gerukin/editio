#import <Foundation/Foundation.h>
#include <string>
// C++ values alongside Objective-C messages.
int main() {
    @autoreleasepool {
        std::string name = "reader";
        NSLog(@"Hello, %s", name.c_str());
    }
}
