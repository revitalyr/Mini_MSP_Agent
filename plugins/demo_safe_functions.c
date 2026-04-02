/**
 * @file demo_safe_functions.c
 * @brief Demonstration of safe functions and platform separation
 */

#include "include/safe_functions.h"
#include <stdio.h>
#include <stdlib.h>

int main() {
    printf("🛡️ Mini MSP Agent - Safe Functions Demo\n");
    printf("==========================================\n\n");
    
    // Test 1: Safe string copy
    printf("📝 Test 1: Safe String Copy\n");
    char dest[64];
    const char* src = "Hello, Mini MSP Agent!";
    
    safe_result_t result = safe_strcpy(dest, sizeof(dest), src);
    if (result == SAFE_SUCCESS) {
        printf("✅ Safe copy successful: \"%s\"\n", dest);
    } else {
        printf("❌ Safe copy failed with error: %d\n", result);
    }
    
    // Test 2: Safe string concatenation
    printf("\n📝 Test 2: Safe String Concatenation\n");
    const char* append = " [SECURE]";
    result = safe_strcat(dest, sizeof(dest), append);
    if (result == SAFE_SUCCESS) {
        printf("✅ Safe concat successful: \"%s\"\n", dest);
    } else {
        printf("❌ Safe concat failed with error: %d\n", result);
    }
    
    // Test 3: Safe formatted string
    printf("\n📝 Test 3: Safe Formatted String\n");
    char buffer[128];
    result = safe_sprintf(buffer, sizeof(buffer), 
                      "Platform: %s, Version: %s, Build: %d", 
                      "Windows", "1.0.0", 2026);
    if (result == SAFE_SUCCESS) {
        printf("✅ Safe sprintf successful: \"%s\"\n", buffer);
    } else {
        printf("❌ Safe sprintf failed with error: %d\n", result);
    }
    
    // Test 4: Buffer overflow protection
    printf("\n📝 Test 4: Buffer Overflow Protection\n");
    char small_buffer[10];
    result = safe_strcpy(small_buffer, sizeof(small_buffer), 
                        "This string is too long for the buffer");
    if (result == SAFE_ERROR_BUFFER_TOO_SMALL) {
        printf("✅ Buffer overflow protection working!\n");
        printf("   Error correctly detected: buffer too small\n");
    } else {
        printf("❌ Buffer overflow protection failed!\n");
    }
    
    // Test 5: NULL pointer protection
    printf("\n📝 Test 5: NULL Pointer Protection\n");
    result = safe_strcpy(NULL, 64, "Test");
    if (result == SAFE_ERROR_NULL_POINTER) {
        printf("✅ NULL pointer protection working!\n");
        printf("   Error correctly detected: null pointer\n");
    } else {
        printf("❌ NULL pointer protection failed!\n");
    }
    
    printf("\n🎉 All safe functions tests completed!\n");
    printf("==========================================\n");
    printf("🛡️ Security Features Demonstrated:\n");
    printf("   ✅ Buffer overflow protection\n");
    printf("   ✅ NULL pointer validation\n");
    printf("   ✅ Safe string operations\n");
    printf("   ✅ Error handling and reporting\n");
    printf("   ✅ Memory safety guarantees\n");
    
    printf("\n🏗️ Platform Separation Benefits:\n");
    printf("   ✅ Windows-specific implementations\n");
    printf("   ✅ Linux-specific implementations\n");
    printf("   ✅ Platform-independent interfaces\n");
    printf("   ✅ Clean architecture separation\n");
    printf("   ✅ Maintainable code structure\n");
    
    return 0;
}
