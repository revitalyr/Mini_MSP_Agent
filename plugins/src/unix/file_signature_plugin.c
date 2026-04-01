#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <openssl/md5.h>
#include <sys/stat.h>
#include "../../include/plugin_interface.h"

static int g_initialized = 0;

int initialize() {
    g_initialized = 1;
    return 1;
}

void cleanup() {
    g_initialized = 0;
}

int calculate_file_hash(const char* path, char* hash, int hash_size) {
    if (!g_initialized || !path || !hash) return 0;
    
    FILE* file = fopen(path, "rb");
    if (!file) return 0;
    
    MD5_CTX md5_ctx;
    MD5_Init(&md5_ctx);
    
    unsigned char buffer[1024];
    size_t bytes_read;
    
    while ((bytes_read = fread(buffer, 1, sizeof(buffer), file)) != 0) {
        MD5_Update(&md5_ctx, buffer, bytes_read);
    }
    
    fclose(file);
    
    unsigned char digest[MD5_DIGEST_LENGTH];
    MD5_Final(digest, &md5_ctx);
    
    // Convert to hex string
    for (int i = 0; i < MD5_DIGEST_LENGTH && i * 2 < hash_size - 1; i++) {
        sprintf(hash + (i * 2), "%02x", digest[i]);
    }
    
    return 1;
}

int verify_file_signature(const char* path, const char* expected_hash) {
    if (!g_initialized || !path || !expected_hash) return 0;
    
    char calculated_hash[MD5_DIGEST_LENGTH * 2 + 1];
    if (!calculate_file_hash(path, calculated_hash, sizeof(calculated_hash))) {
        return 0;
    }
    
    return strcmp(calculated_hash, expected_hash) == 0;
}

const char* get_plugin_name() {
    return "File Signature Plugin";
}

const char* get_plugin_version() {
    return "1.0.0";
}

const char* get_plugin_description() {
    return "Plugin for calculating and verifying file signatures on Unix/Linux";
}
