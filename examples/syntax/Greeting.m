#import <Foundation/Foundation.h>
// Objective-C string literals and messages.
int main(void) {
    @autoreleasepool {
        NSString *name = @"reader";
        NSLog(@"Hello, %@!", name);
    }
    return 0;
}
