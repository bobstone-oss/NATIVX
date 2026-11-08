// Archivo: crates/nx_hal_sensors/src/ios_native_bridge.m
#import <UIKit/UIKit.h>
#import <Metal/Metal.h>
#import <CoreMotion/CoreMotion.h>
#import <CoreHaptics/CoreHaptics.h>

extern void __nativx_core_motion_callback(float x, float y, float z);

static CMMotionManager *motionManager = nil;
static CHHapticEngine *hapticEngine = nil;

void NativX_StartCoreMotion(void (*callback)(float, float, float)) {
    motionManager = [[CMMotionManager alloc] init];
    if ([motionManager isGyroAvailable]) {
        motionManager.gyroUpdateInterval = 1.0 / 120.0; // 120 Hz nativos
        [motionManager startGyroUpdatesToQueue:[NSOperationQueue currentQueue]
                                   withHandler:^(CMGyroData *gyroData, NSError *error) {
            if (gyroData) {
                __nativx_core_motion_callback(
                    (float)gyroData.rotationRate.x,
                    (float)gyroData.rotationRate.y,
                    (float)gyroData.rotationRate.z
                );
            }
        }];
    }
}

void NativX_TriggerHaptic(float intensity) {
    if (!hapticEngine) {
        CHHapticCapability *caps = CHHapticEngine.capabilitiesForHardware;
        if (caps.supportsHaptics) {
            hapticEngine = [[CHHapticEngine alloc] initAndReturnError:nil];
            [hapticEngine startAndReturnError:nil];
        }
    }
    if (hapticEngine) {
        CHHapticEventParameter *intensityParam = [[CHHapticEventParameter alloc] initWithParameterID:CHHapticEventParameterIDHapticIntensity value:intensity];
        CHHapticEvent *event = [[CHHapticEvent alloc] initWithEventType:CHHapticEventTypeHapticTransient parameters:@[intensityParam] relativeTime:0];
        CHHapticPattern *pattern = [[CHHapticPattern alloc] initWithEvents:@[event] parameters:@[] error:nil];
        id<CHHapticPatternPlayer> player = [hapticEngine createPlayerWithPattern:pattern error:nil];
        [player startAtTime:0 error:nil];
    }
}
