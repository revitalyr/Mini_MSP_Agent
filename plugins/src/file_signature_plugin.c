/**
 * @file file_signature_plugin.c
 * @brief File Signature Plugin for Mini MSP Agent
 * 
 * Provides file signature calculation and verification including
 * MD5, SHA-1, SHA-256 hashes and file integrity checks.
 * 
 * @author Mini MSP Agent Team
 * @version 1.0.0
 * @date 2026
 */

#include "../include/plugin_interface.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

#ifdef _WIN32
#include <windows.h>
#include <wincrypt.h>
#pragma comment(lib, "crypt32.lib")
#else
#include <openssl/md5.h>
#include <openssl/sha.h>
#include <openssl/evp.h>
#endif

// Plugin information
static plugin_info_t file_signature_plugin_info = {
    .name = "file_signature",
    .version = "1.0.0",
    .description = "Calculates and verifies file signatures and hashes"
};

/**
 * @brief File signature structure
 */
typedef struct {
    char path[512];
    char md5[33];
    char sha1[41];
    char sha256[65];
    uint64_t file_size;
    uint64_t modified_time;
    bool signature_valid;
} file_signature_t;

/**
 * @brief Hash calculation options
 */
typedef struct {
    bool calculate_md5;
    bool calculate_sha1;
    bool calculate_sha256;
    bool verify_integrity;
} hash_options_t;

// Plugin implementation
static bool file_signature_init(void) {
#ifdef _WIN32
    // Initialize Windows Crypto API
    // This is typically not needed for basic hash functions
#else
    // Initialize OpenSSL (usually not needed for modern OpenSSL)
#endif
    return true;
}

static void file_signature_cleanup(void) {
#ifdef _WIN32
    // Cleanup Windows Crypto API resources
#else
    // Cleanup OpenSSL resources
#endif
}

static plugin_info_t* file_signature_get_plugin_info(void) {
    return &file_signature_plugin_info;
}

static bool file_signature_get_system_metrics(system_metrics_t* metrics) {
    // Not applicable for file signature plugin
    return false;
}

static bool file_signature_get_processes(process_info_t** processes, size_t* count) {
    // Not applicable for file signature plugin
    return false;
}

static bool file_signature_execute_command(const char* command, command_result_t* result) {
    // Not applicable for file signature plugin
    return false;
}

static bool file_signature_read_file(const char* path, file_content_t* content) {
    // Not applicable for file signature plugin
    return false;
}

static bool file_signature_get_system_info(system_info_t* info) {
    // Not applicable for file signature plugin
    return false;
}

#ifdef _WIN32
/**
 * @brief Calculate hash using Windows Crypto API
 */
static bool calculate_hash_windows(const char* path, const char* algorithm, char* hash_output, size_t hash_size) {
    HCRYPTPROV hProv = 0;
    HCRYPTHASH hHash = 0;
    HANDLE hFile = INVALID_HANDLE_VALUE;
    DWORD bytesRead;
    BYTE buffer[4096];
    bool success = false;
    
    // Get crypto provider
    if (!CryptAcquireContext(&hProv, NULL, NULL, PROV_RSA_FULL, CRYPT_VERIFYCONTEXT)) {
        return false;
    }
    
    // Create hash object
    ALG_ID algId;
    if (strcmp(algorithm, "MD5") == 0) {
        algId = CALG_MD5;
    } else if (strcmp(algorithm, "SHA1") == 0) {
        algId = CALG_SHA1;
    } else if (strcmp(algorithm, "SHA256") == 0) {
        algId = CALG_SHA_256;
    } else {
        CryptReleaseContext(hProv, 0);
        return false;
    }
    
    if (!CryptCreateHash(hProv, algId, 0, 0, &hHash)) {
        CryptReleaseContext(hProv, 0);
        return false;
    }
    
    // Open file
    hFile = CreateFileA(path, GENERIC_READ, FILE_SHARE_READ, NULL, 
                        OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    if (hFile == INVALID_HANDLE_VALUE) {
        CryptDestroyHash(hHash);
        CryptReleaseContext(hProv, 0);
        return false;
    }
    
    // Hash file data
    while (ReadFile(hFile, buffer, sizeof(buffer), &bytesRead, NULL) && bytesRead > 0) {
        if (!CryptHashData(hHash, buffer, bytesRead, 0)) {
            break;
        }
    }
    
    // Get hash value
    DWORD hashLen = 0;
    DWORD hashLenSize = sizeof(DWORD);
    if (CryptGetHashParam(hHash, HP_HASHSIZE, (BYTE*)&hashLen, &hashLenSize, 0)) {
        BYTE* hashValue = (BYTE*)malloc(hashLen);
        if (hashValue && CryptGetHashParam(hHash, HP_HASHVAL, hashValue, &hashLen, 0)) {
            // Convert to hex string
            for (DWORD i = 0; i < hashLen; i++) {
                sprintf(hash_output + (i * 2), "%02x", hashValue[i]);
            }
            hash_output[hashLen * 2] = '\0';
            success = true;
        }
        if (hashValue) free(hashValue);
    }
    
    // Cleanup
    CloseHandle(hFile);
    CryptDestroyHash(hHash);
    CryptReleaseContext(hProv, 0);
    
    return success;
}

#else
/**
 * @brief Calculate hash using OpenSSL
 */
static bool calculate_hash_openssl(const char* path, const char* algorithm, char* hash_output, size_t hash_size) {
    FILE* fp = fopen(path, "rb");
    if (!fp) {
        return false;
    }
    
    EVP_MD_CTX* mdctx = EVP_MD_CTX_new();
    if (!mdctx) {
        fclose(fp);
        return false;
    }
    
    const EVP_MD* md;
    if (strcmp(algorithm, "MD5") == 0) {
        md = EVP_md5();
    } else if (strcmp(algorithm, "SHA1") == 0) {
        md = EVP_sha1();
    } else if (strcmp(algorithm, "SHA256") == 0) {
        md = EVP_sha256();
    } else {
        EVP_MD_CTX_free(mdctx);
        fclose(fp);
        return false;
    }
    
    if (!EVP_DigestInit_ex(mdctx, md, NULL)) {
        EVP_MD_CTX_free(mdctx);
        fclose(fp);
        return false;
    }
    
    BYTE buffer[4096];
    size_t bytesRead;
    bool success = false;
    
    while ((bytesRead = fread(buffer, 1, sizeof(buffer), fp)) > 0) {
        if (!EVP_DigestUpdate(mdctx, buffer, bytesRead)) {
            break;
        }
    }
    
    unsigned char hash[EVP_MAX_MD_SIZE];
    unsigned int hashLen;
    
    if (EVP_DigestFinal_ex(mdctx, hash, &hashLen)) {
        // Convert to hex string
        for (unsigned int i = 0; i < hashLen; i++) {
            sprintf(hash_output + (i * 2), "%02x", hash[i]);
        }
        hash_output[hashLen * 2] = '\0';
        success = true;
    }
    
    EVP_MD_CTX_free(mdctx);
    fclose(fp);
    
    return success;
}
#endif

/**
 * @brief Calculate file signature
 */
static bool calculate_file_signature(const char* path, file_signature_t* signature, const hash_options_t* options) {
    if (!path || !signature) return false;
    
    memset(signature, 0, sizeof(file_signature_t));
    strncpy(signature->path, path, sizeof(signature->path) - 1);
    
    // Get file metadata
#ifdef _WIN32
    WIN32_FILE_ATTRIBUTE_DATA fileData;
    if (!GetFileAttributesExA(path, GetFileExInfoStandard, &fileData)) {
        return false;
    }
    
    signature->file_size = ((uint64_t)fileData.nFileSizeHigh << 32) | fileData.nFileSizeLow;
    
    ULARGE_INTEGER ull;
    ull.LowPart = fileData.ftLastWriteTime.dwLowDateTime;
    ull.HighPart = fileData.ftLastWriteTime.dwHighDateTime;
    signature->modified_time = (ull.QuadPart - 116444736000000000ULL) / 10000000ULL;
#else
    struct stat st;
    if (stat(path, &st) == -1) {
        return false;
    }
    
    signature->file_size = st.st_size;
    signature->modified_time = st.st_mtime;
#endif
    
    // Calculate hashes
    bool success = true;
    
    if (!options || options->calculate_md5) {
#ifdef _WIN32
        success &= calculate_hash_windows(path, "MD5", signature->md5, sizeof(signature->md5));
#else
        success &= calculate_hash_openssl(path, "MD5", signature->md5, sizeof(signature->md5));
#endif
    }
    
    if (!options || options->calculate_sha1) {
#ifdef _WIN32
        success &= calculate_hash_windows(path, "SHA1", signature->sha1, sizeof(signature->sha1));
#else
        success &= calculate_hash_openssl(path, "SHA1", signature->sha1, sizeof(signature->sha1));
#endif
    }
    
    if (!options || options->calculate_sha256) {
#ifdef _WIN32
        success &= calculate_hash_windows(path, "SHA256", signature->sha256, sizeof(signature->sha256));
#else
        success &= calculate_hash_openssl(path, "SHA256", signature->sha256, sizeof(signature->sha256));
#endif
    }
    
    signature->signature_valid = success;
    return success;
}

/**
 * @brief Verify file signature
 */
static bool verify_file_signature(const char* path, const file_signature_t* expected_signature) {
    if (!path || !expected_signature) return false;
    
    file_signature_t current_signature;
    hash_options_t options = {
        .calculate_md5 = true,
        .calculate_sha1 = true,
        .calculate_sha256 = true,
        .verify_integrity = true
    };
    
    if (!calculate_file_signature(path, &current_signature, &options)) {
        return false;
    }
    
    // Compare hashes
    bool md5_match = (strlen(expected_signature->md5) == 0) || 
                     (strcmp(current_signature.md5, expected_signature->md5) == 0);
    
    bool sha1_match = (strlen(expected_signature->sha1) == 0) || 
                      (strcmp(current_signature.sha1, expected_signature->sha1) == 0);
    
    bool sha256_match = (strlen(expected_signature->sha256) == 0) || 
                        (strcmp(current_signature.sha256, expected_signature->sha256) == 0);
    
    bool size_match = (current_signature.file_size == expected_signature->file_size);
    
    return md5_match && sha1_match && sha256_match && size_match;
}

/**
 * @brief Calculate string hash
 */
static bool calculate_string_hash(const char* input, const char* algorithm, char* hash_output, size_t hash_size) {
    if (!input || !algorithm || !hash_output) return false;
    
#ifdef _WIN32
    HCRYPTPROV hProv = 0;
    HCRYPTHASH hHash = 0;
    bool success = false;
    
    if (!CryptAcquireContext(&hProv, NULL, NULL, PROV_RSA_FULL, CRYPT_VERIFYCONTEXT)) {
        return false;
    }
    
    ALG_ID algId;
    if (strcmp(algorithm, "MD5") == 0) {
        algId = CALG_MD5;
    } else if (strcmp(algorithm, "SHA1") == 0) {
        algId = CALG_SHA1;
    } else if (strcmp(algorithm, "SHA256") == 0) {
        algId = CALG_SHA_256;
    } else {
        CryptReleaseContext(hProv, 0);
        return false;
    }
    
    if (!CryptCreateHash(hProv, algId, 0, 0, &hHash)) {
        CryptReleaseContext(hProv, 0);
        return false;
    }
    
    if (CryptHashData(hHash, (const BYTE*)input, strlen(input), 0)) {
        DWORD hashLen = 0;
        DWORD hashLenSize = sizeof(DWORD);
        if (CryptGetHashParam(hHash, HP_HASHSIZE, (BYTE*)&hashLen, &hashLenSize, 0)) {
            BYTE* hashValue = (BYTE*)malloc(hashLen);
            if (hashValue && CryptGetHashParam(hHash, HP_HASHVAL, hashValue, &hashLen, 0)) {
                for (DWORD i = 0; i < hashLen; i++) {
                    sprintf(hash_output + (i * 2), "%02x", hashValue[i]);
                }
                hash_output[hashLen * 2] = '\0';
                success = true;
            }
            if (hashValue) free(hashValue);
        }
    }
    
    CryptDestroyHash(hHash);
    CryptReleaseContext(hProv, 0);
    
    return success;
#else
    EVP_MD_CTX* mdctx = EVP_MD_CTX_new();
    if (!mdctx) return false;
    
    const EVP_MD* md;
    if (strcmp(algorithm, "MD5") == 0) {
        md = EVP_md5();
    } else if (strcmp(algorithm, "SHA1") == 0) {
        md = EVP_sha1();
    } else if (strcmp(algorithm, "SHA256") == 0) {
        md = EVP_sha256();
    } else {
        EVP_MD_CTX_free(mdctx);
        return false;
    }
    
    if (!EVP_DigestInit_ex(mdctx, md, NULL)) {
        EVP_MD_CTX_free(mdctx);
        return false;
    }
    
    if (!EVP_DigestUpdate(mdctx, input, strlen(input))) {
        EVP_MD_CTX_free(mdctx);
        return false;
    }
    
    unsigned char hash[EVP_MAX_MD_SIZE];
    unsigned int hashLen;
    
    bool success = false;
    if (EVP_DigestFinal_ex(mdctx, hash, &hashLen)) {
        for (unsigned int i = 0; i < hashLen; i++) {
            sprintf(hash_output + (i * 2), "%02x", hash[i]);
        }
        hash_output[hashLen * 2] = '\0';
        success = true;
    }
    
    EVP_MD_CTX_free(mdctx);
    return success;
#endif
}

static void file_signature_free_memory(void* ptr) {
    if (ptr) {
        free(ptr);
    }
}

// Plugin interface
static plugin_interface_t file_signature_interface = {
    .get_plugin_info = file_signature_get_plugin_info,
    .init = file_signature_init,
    .cleanup = file_signature_cleanup,
    .get_system_metrics = file_signature_get_system_metrics,
    .get_processes = file_signature_get_processes,
    .execute_command = file_signature_execute_command,
    .read_file = file_signature_read_file,
    .get_system_info = file_signature_get_system_info,
    .free_memory = file_signature_free_memory
};

// Plugin entry point
PLUGIN_EXPORT plugin_interface_t* PLUGIN_CALL get_plugin_interface(void) {
    return &file_signature_interface;
}
