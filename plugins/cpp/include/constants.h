#ifndef CONSTANTS_H
#define CONSTANTS_H

#ifdef __cplusplus
extern "C" {
#endif

// =============================================================================
// CONSTANTS AND STRING LITERALS
// =============================================================================
//
// This header defines all hardcoded constants, string lengths, and
// configuration values used across the plugin system.
//
// Naming convention: MAX_<PURPOSE>_<TYPE>_LEN or <DOMAIN>_<CONSTANT>
// =============================================================================

// -----------------------------------------------------------------------------
// String Length Constants
// -----------------------------------------------------------------------------

#define kMaxPermissionLen         16  // Maximum length for permission string
#define kMaxEventMsgLen          64  // Maximum length for event message
#define kMaxCodecNameLen         16  // Maximum length for codec name
#define kMaxEncodingLen          32  // Maximum length for encoding name
#define kMaxStatusLen            64  // Maximum length for status message
#define kMaxPatternLen          256  // Maximum length for file pattern
#define kMaxHostnameLen         256  // Maximum length for hostname
#define kMaxOsTypeLen            64  // Maximum length for OS type
#define kMaxOsVersionLen        128  // Maximum length for OS version
#define kMaxErrorMsgLen         512  // Maximum length for error message
#define kMaxCommandOutputLen    512  // Maximum length for command output

// -----------------------------------------------------------------------------
// Numeric Constants
// -----------------------------------------------------------------------------

#define kDefaultMaxDepth         10  // Default maximum directory scan depth
#define kDefaultScanInterval      5  // Default scan interval in seconds
#define kMaxMemoryUsageMb      512  // Maximum memory usage in MB
#define kDefaultHeartbeatSec     30  // Default heartbeat interval in seconds

// -----------------------------------------------------------------------------
// String Literals
// -----------------------------------------------------------------------------

#define kDefaultEncoding      "utf-8"  // Default text encoding
#define kDefaultCodec         "h264"  // Default video codec
#define kDefaultStatus         "ok"   // Default status message
#define kErrorStatus        "error"   // Error status
#define kSuccessStatus   "success"   // Success status

// -----------------------------------------------------------------------------
// Plugin Configuration Constants
// -----------------------------------------------------------------------------

#define kPluginApiVersion   "2.0.0"  // Plugin API version
#define kPluginInterfaceVersion  1   // Plugin interface version

#ifdef __cplusplus
}
#endif

#endif // CONSTANTS_H
