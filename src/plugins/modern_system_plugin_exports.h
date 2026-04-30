
#ifndef MODERN_SYSTEM_PLUGIN_API_H
#define MODERN_SYSTEM_PLUGIN_API_H

#ifdef MODERN_SYSTEM_PLUGIN_STATIC
#  define MODERN_SYSTEM_PLUGIN_API
#  define MODERNSYSTEMPLUGIN_NO_EXPORT
#else
#  ifndef MODERN_SYSTEM_PLUGIN_API
#    ifdef ModernSystemPlugin_EXPORTS
        /* We are building this library */
#      define MODERN_SYSTEM_PLUGIN_API __declspec(dllexport)
#    else
        /* We are using this library */
#      define MODERN_SYSTEM_PLUGIN_API __declspec(dllimport)
#    endif
#  endif

#  ifndef MODERNSYSTEMPLUGIN_NO_EXPORT
#    define MODERNSYSTEMPLUGIN_NO_EXPORT 
#  endif
#endif

#ifndef MODERNSYSTEMPLUGIN_DEPRECATED
#  define MODERNSYSTEMPLUGIN_DEPRECATED __declspec(deprecated)
#endif

#ifndef MODERNSYSTEMPLUGIN_DEPRECATED_EXPORT
#  define MODERNSYSTEMPLUGIN_DEPRECATED_EXPORT MODERN_SYSTEM_PLUGIN_API MODERNSYSTEMPLUGIN_DEPRECATED
#endif

#ifndef MODERNSYSTEMPLUGIN_DEPRECATED_NO_EXPORT
#  define MODERNSYSTEMPLUGIN_DEPRECATED_NO_EXPORT MODERNSYSTEMPLUGIN_NO_EXPORT MODERNSYSTEMPLUGIN_DEPRECATED
#endif

/* NOLINTNEXTLINE(readability-avoid-unconditional-preprocessor-if) */
#if 0 /* DEFINE_NO_DEPRECATED */
#  ifndef MODERNSYSTEMPLUGIN_NO_DEPRECATED
#    define MODERNSYSTEMPLUGIN_NO_DEPRECATED
#  endif
#endif

#endif /* MODERN_SYSTEM_PLUGIN_API_H */
