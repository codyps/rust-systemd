use super::c_int;

pub const _SD_BUS_MESSAGE_TYPE_INVALID: c_int = 0;
pub const SD_BUS_MESSAGE_METHOD_CALL: c_int = 1;
pub const SD_BUS_MESSAGE_METHOD_RETURN: c_int = 2;
pub const SD_BUS_MESSAGE_METHOD_ERROR: c_int = 3;
pub const SD_BUS_MESSAGE_SIGNAL: c_int = 4;
pub const _SD_BUS_MESSAGE_TYPE_MAX: c_int = 5;

/*
        _SD_BUS_TYPE_INVALID         = 0,
        SD_BUS_TYPE_BYTE             = 'y',
        SD_BUS_TYPE_BOOLEAN          = 'b',
        SD_BUS_TYPE_INT16            = 'n',
        SD_BUS_TYPE_UINT16           = 'q',
        SD_BUS_TYPE_INT32            = 'i',
        SD_BUS_TYPE_UINT32           = 'u',
        SD_BUS_TYPE_INT64            = 'x',
        SD_BUS_TYPE_UINT64           = 't',
        SD_BUS_TYPE_DOUBLE           = 'd',
        SD_BUS_TYPE_STRING           = 's',
        SD_BUS_TYPE_OBJECT_PATH      = 'o',
        SD_BUS_TYPE_SIGNATURE        = 'g',
        SD_BUS_TYPE_UNIX_FD          = 'h',
        SD_BUS_TYPE_ARRAY            = 'a',
        SD_BUS_TYPE_VARIANT          = 'v',
        SD_BUS_TYPE_STRUCT           = 'r', /* not actually used in signatures */
        SD_BUS_TYPE_STRUCT_BEGIN     = '(',
        SD_BUS_TYPE_STRUCT_END       = ')',
        SD_BUS_TYPE_DICT_ENTRY       = 'e', /* not actually used in signatures */
        SD_BUS_TYPE_DICT_ENTRY_BEGIN = '{',
        SD_BUS_TYPE_DICT_ENTRY_END   = '}'
*/

/*
 *

#define SD_BUS_ERROR_FAILED                     "org.freedesktop.DBus.Error.Failed"
#define SD_BUS_ERROR_NO_MEMORY                  "org.freedesktop.DBus.Error.NoMemory"
#define SD_BUS_ERROR_SERVICE_UNKNOWN            "org.freedesktop.DBus.Error.ServiceUnknown"
#define SD_BUS_ERROR_NAME_HAS_NO_OWNER          "org.freedesktop.DBus.Error.NameHasNoOwner"
#define SD_BUS_ERROR_NO_REPLY                   "org.freedesktop.DBus.Error.NoReply"
#define SD_BUS_ERROR_IO_ERROR                   "org.freedesktop.DBus.Error.IOError"
#define SD_BUS_ERROR_BAD_ADDRESS                "org.freedesktop.DBus.Error.BadAddress"
#define SD_BUS_ERROR_NOT_SUPPORTED              "org.freedesktop.DBus.Error.NotSupported"
#define SD_BUS_ERROR_LIMITS_EXCEEDED            "org.freedesktop.DBus.Error.LimitsExceeded"
#define SD_BUS_ERROR_ACCESS_DENIED              "org.freedesktop.DBus.Error.AccessDenied"
#define SD_BUS_ERROR_AUTH_FAILED                "org.freedesktop.DBus.Error.AuthFailed"
#define SD_BUS_ERROR_NO_SERVER                  "org.freedesktop.DBus.Error.NoServer"
#define SD_BUS_ERROR_TIMEOUT                    "org.freedesktop.DBus.Error.Timeout"
#define SD_BUS_ERROR_NO_NETWORK                 "org.freedesktop.DBus.Error.NoNetwork"
#define SD_BUS_ERROR_ADDRESS_IN_USE             "org.freedesktop.DBus.Error.AddressInUse"
#define SD_BUS_ERROR_DISCONNECTED               "org.freedesktop.DBus.Error.Disconnected"
#define SD_BUS_ERROR_INVALID_ARGS               "org.freedesktop.DBus.Error.InvalidArgs"
#define SD_BUS_ERROR_FILE_NOT_FOUND             "org.freedesktop.DBus.Error.FileNotFound"
#define SD_BUS_ERROR_FILE_EXISTS                "org.freedesktop.DBus.Error.FileExists"
#define SD_BUS_ERROR_UNKNOWN_METHOD             "org.freedesktop.DBus.Error.UnknownMethod"
#define SD_BUS_ERROR_UNKNOWN_OBJECT             "org.freedesktop.DBus.Error.UnknownObject"
#define SD_BUS_ERROR_UNKNOWN_INTERFACE          "org.freedesktop.DBus.Error.UnknownInterface"
#define SD_BUS_ERROR_UNKNOWN_PROPERTY           "org.freedesktop.DBus.Error.UnknownProperty"
#define SD_BUS_ERROR_PROPERTY_READ_ONLY         "org.freedesktop.DBus.Error.PropertyReadOnly"
#define SD_BUS_ERROR_UNIX_PROCESS_ID_UNKNOWN    "org.freedesktop.DBus.Error.UnixProcessIdUnknown"
#define SD_BUS_ERROR_INVALID_SIGNATURE          "org.freedesktop.DBus.Error.InvalidSignature"
#define SD_BUS_ERROR_INCONSISTENT_MESSAGE       "org.freedesktop.DBus.Error.InconsistentMessage"
#define SD_BUS_ERROR_MATCH_RULE_NOT_FOUND       "org.freedesktop.DBus.Error.MatchRuleNotFound"
#define SD_BUS_ERROR_MATCH_RULE_INVALID         "org.freedesktop.DBus.Error.MatchRuleInvalid"
#define SD_BUS_ERROR_INTERACTIVE_AUTHORIZATION_REQUIRED \
                                                "org.freedesktop.DBus.Error.InteractiveAuthorizationRequired"
*/
