//! Power Configuration
//!
//! This module configures the PWR unit to provide the core voltage `VCORE`. The
//! voltage scaling mode is VOS1 (High Performance) by default, but VOS2, VOS3
//! and [VOS0](#boost-mode-vos0) can also be selected.
//!
//! When the system starts up, it is in Run* mode. After the call to
//! `freeze`, it will be in Run mode. See RM0433 Rev 7 Section 6.6.1
//! "System/D3 domain modes".
//!
//! # Example
//!
//! You can also find a simple example [here](https://github.com/stm32-rs/stm32h7xx-hal/blob/master/examples/vos0.rs).
//!
//! ```rust
//!     let dp = pac::Peripherals::take().unwrap();
//!
//!     let pwr = dp.PWR.constrain();
//!     let pwrcfg = pwr.freeze();
//!
//!     assert_eq!(pwrcfg.vos(), VoltageScale::Scale1);
//! ```
//!
//! # SMPS
//!
//! Some parts include an integrated Switched Mode Power Supply (SMPS)
//! to supply VCORE. For these parts, the method of supplying VCORE
//! can be specified. Refer to RM0399 Rev 3 Table 32. for a more
//! detailed descriptions of the possible modes.
//!
//! - Low Dropout Regulator [ldo](Pwr#ldo)
//! - Switch Mode Power Supply [smps](Pwr#smps)
//! - Bypass [bypass](Pwr#pypass)
//! - SMPS Output at 1.8V, then LDO [smps_1v8_feeds_ldo](Pwr#smps_1v8_feeds_ldo)
//! - SMPS Output at 2.5V, then LDO [smps_2v5_feeds_ldo](Pwr#smps_2v5_feeds_ldo)
//!
//! **Note**: Specifying the wrong mode for your hardware will cause
//! undefined results.
//!
//! ```rust
//!     let dp = pac::Peripherals::take().unwrap();
//!
//!     let pwr = dp.PWR.constrain();
//!     let pwrcfg = pwr.smps().freeze();
//!
//!     assert_eq!(pwrcfg.vos(), VoltageScale::Scale1);
//! ```
//!
//! The VCORE supply configuration can only be set once after each
//! POR, and this is enforced by hardware. If you add or change the
//! power supply method, `freeze` will panic until you power on reset
//! your board.
//!
//! # Boost Mode (VOS0)
//!
//! Some parts have a Boost Mode that allows higher clock speeds. This can be
//! selected using the `.vos0(..)` builder method. The following parts are supported:
//!
//! | Parts | Reference Manual | Maximum Core Clock with VOS0 |
//! | --- | --- | ---
//! | stm32h743/753/750 | RM0433 | 480MHz [^revv]
//! | stm32h747/757 | RM0399 | 480MHz
//! | stm32h7a3/7b3/7b0 | RM0455 | VOS0 not supported
//! | stm32h725/735 | RM0468 | 520MHz [^rm0468ecc]
//!
//! [^revv]: Revision V and later parts only
//!
//! [^rm0468ecc]: These parts allow up to 550MHz by setting an additional bit in
//! User Register 18, but this is not supported through the HAL.
//!
//! ## Examples
//!
//! - [Enable VOS0](https://github.com/stm32-rs/stm32h7xx-hal/blob/master/examples/vos0.rs)
//! - [Enable USB regulator](https://github.com/stm32-rs/stm32h7xx-hal/blob/master/examples/usb_serial.rs)

use crate::rcc::backup::BackupREC;
use crate::stm32::PWR;
#[cfg(all(feature = "revision_v", feature = "rm0468"))]
use crate::stm32::SYSCFG;
#[cfg(all(
    feature = "revision_v",
    any(feature = "rm0433", feature = "rm0399")
))]
use crate::stm32::{RCC, SYSCFG};

#[cfg(all(
    feature = "rm0433",
    any(feature = "smps", feature = "example-smps")
))]
compile_error!("SMPS configuration fields are not available for RM0433 parts");

/// Extension trait that constrains the `PWR` peripheral
pub trait PwrExt {
    fn constrain(self) -> Pwr;
}

impl PwrExt for PWR {
    fn constrain(self) -> Pwr {
        Pwr {
            rb: self,
            #[cfg(any(feature = "smps"))]
            supply_configuration: SupplyConfiguration::Default,
            target_vos: VoltageScale::Scale1,
            backup_regulator: false,
        }
    }
}

/// Constrained PWR peripheral
///
/// Generated by calling `constrain` on the PAC's PWR peripheral.
pub struct Pwr {
    pub(crate) rb: PWR,
    #[cfg(any(feature = "smps"))]
    supply_configuration: SupplyConfiguration,
    target_vos: VoltageScale,
    backup_regulator: bool,
}

/// Voltage Scale
///
/// Represents the voltage range feeding the CPU core. The maximum core
/// clock frequency depends on this value.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum VoltageScale {
    /// VOS 0 range VCORE 1.26V - 1.40V
    Scale0,
    /// VOS 1 range VCORE 1.15V - 1.26V
    Scale1,
    /// VOS 2 range VCORE 1.05V - 1.15V
    Scale2,
    /// VOS 3 range VCORE 0.95V - 1.05V
    Scale3,
}

/// Power Configuration
///
/// Generated when the PWR peripheral is frozen. The existence of this
/// value indicates that the voltage scaling configuration can no
/// longer be changed.
pub struct PowerConfiguration {
    pub(crate) vos: VoltageScale,
    pub(crate) backup: Option<BackupREC>,
}

impl PowerConfiguration {
    /// Gets the `VoltageScale` which was configured by `Pwr::freeze()`.
    pub fn vos(&self) -> VoltageScale {
        self.vos
    }

    pub fn backup(&mut self) -> Option<BackupREC> {
        self.backup.take()
    }
}

/// SMPS Supply Configuration - Dual Core parts
///
/// Refer to RM0399 Rev 3 Table 32.
#[cfg(any(feature = "smps"))]
enum SupplyConfiguration {
    Default = 0,
    LDOSupply,
    DirectSMPS,
    SMPSFeedsIntoLDO1V8,
    SMPSFeedsIntoLDO2V5,
    // External SMPS loads not supported
    Bypass,
}

#[cfg(any(feature = "smps"))]
macro_rules! supply_configuration_setter {
    ($($config:ident: $name:ident, $doc:expr,)*) => {
        $(
            #[doc=$doc]
            #[must_use]
            pub fn $name(mut self) -> Self {
                self.supply_configuration = SupplyConfiguration::$config;
                self
            }
        )*
    };
}

// smpslevel / sdlevel macro
#[cfg(all(feature = "smps", not(feature = "rm0455")))]
macro_rules! smps_level {
    ($e:expr) => {
        $e.sdlevel()
    };
}
#[cfg(all(feature = "smps", feature = "rm0455"))]
macro_rules! smps_level {
    ($e:expr) => {
        $e.smpslevel()
    };
}
#[cfg(all(feature = "smps", not(feature = "rm0455")))]
macro_rules! smps_en {
    ($e:expr) => {
        $e.sden()
    };
}
#[cfg(all(feature = "smps", feature = "rm0455"))]
macro_rules! smps_en {
    ($e:expr) => {
        $e.smpsen()
    };
}
#[cfg(not(feature = "rm0455"))]
macro_rules! d3cr {
    ($e:expr) => {
        $e.d3cr
    };
}
#[cfg(feature = "rm0455")]
macro_rules! d3cr {
    ($e:expr) => {
        $e.srdcr
    };
}

/// Internal power methods
impl Pwr {
    /// Verify that the lower byte of CR3 reads as written
    #[cfg(any(feature = "smps"))]
    fn verify_supply_configuration(&self) {
        use SupplyConfiguration::*;
        let error = "Values in lower byte of PWR.CR3 do not match the \
                     configured power mode. These values can only be set \
                     once for each POR (Power-on-Reset). Try removing power \
                     to your board.";

        match self.supply_configuration {
            LDOSupply => {
                assert!(
                    smps_en!(self.rb.cr3.read()).bit_is_clear(),
                    "{}",
                    error
                );
                assert!(self.rb.cr3.read().ldoen().bit_is_set(), "{}", error);
            }
            DirectSMPS => {
                assert!(smps_en!(self.rb.cr3.read()).bit_is_set(), "{}", error);
                assert!(self.rb.cr3.read().ldoen().bit_is_clear(), "{}", error);
            }
            SMPSFeedsIntoLDO1V8 => {
                assert!(smps_en!(self.rb.cr3.read()).bit_is_set(), "{}", error);
                assert!(self.rb.cr3.read().ldoen().bit_is_clear(), "{}", error);
                assert!(
                    smps_level!(self.rb.cr3.read()).bits() == 1,
                    "{}",
                    error
                );
            }
            SMPSFeedsIntoLDO2V5 => {
                assert!(smps_en!(self.rb.cr3.read()).bit_is_set(), "{}", error);
                assert!(self.rb.cr3.read().ldoen().bit_is_clear(), "{}", error);
                assert!(
                    smps_level!(self.rb.cr3.read()).bits() == 2,
                    "{}",
                    error
                );
            }
            Bypass => {
                assert!(
                    smps_en!(self.rb.cr3.read()).bit_is_clear(),
                    "{}",
                    error
                );
                assert!(self.rb.cr3.read().ldoen().bit_is_clear(), "{}", error);
                assert!(self.rb.cr3.read().bypass().bit_is_set(), "{}", error);
            }
            Default => {} // Default configuration is NOT verified
        }
    }

    /// Transition between voltage scaling levels using the D3CR / SRDCR
    /// register
    ///
    /// Does NOT implement overdrive (back-bias)
    fn voltage_scaling_transition(&self, new_scale: VoltageScale) {
        d3cr!(self.rb).write(|w| unsafe {
            // Manually set field values for each family
            w.vos().bits(
                #[cfg(any(feature = "rm0433", feature = "rm0399"))]
                match new_scale {
                    // RM0433 Rev 7 6.8.6
                    VoltageScale::Scale3 => 0b01,
                    VoltageScale::Scale2 => 0b10,
                    VoltageScale::Scale1 => 0b11,
                    _ => unimplemented!(),
                },
                #[cfg(feature = "rm0455")]
                match new_scale {
                    // RM0455 Rev 3 6.8.6
                    VoltageScale::Scale3 => 0b00,
                    VoltageScale::Scale2 => 0b01,
                    VoltageScale::Scale1 => 0b10,
                    VoltageScale::Scale0 => 0b11,
                },
                #[cfg(feature = "rm0468")]
                match new_scale {
                    // RM0468 Rev 2 6.8.6
                    VoltageScale::Scale0 => 0b00,
                    VoltageScale::Scale3 => 0b01,
                    VoltageScale::Scale2 => 0b10,
                    VoltageScale::Scale1 => 0b11,
                },
            )
        });
        while d3cr!(self.rb).read().vosrdy().bit_is_clear() {}
    }

    /// Returns a reference to the inner peripheral
    pub fn inner(&self) -> &PWR {
        &self.rb
    }

    /// Returns a mutable reference to the inner peripheral
    pub fn inner_mut(&mut self) -> &mut PWR {
        &mut self.rb
    }
}

/// Builder methods
impl Pwr {
    #[cfg(any(feature = "smps"))]
    supply_configuration_setter! {
        LDOSupply: ldo, "VCORE power domains supplied from the LDO. \
                         LDO voltage adjusted by VOS. \
                         LDO power mode will follow the system \
                         low-power mode.",
        DirectSMPS: smps, "VCORE power domains are supplied from the \
                           SMPS step-down converter. SMPS output voltage \
                           adjusted by VOS. SMPS power mode will follow \
                           the system low-power mode",
        Bypass: bypass, "VCORE is supplied from an external source",
        SMPSFeedsIntoLDO1V8:
        smps_1v8_feeds_ldo, "VCORE power domains supplied from the LDO. \
                         LDO voltage adjusted by VOS. \
                         LDO power mode will follow the system \
                         low-power mode. SMPS output voltage set to \
                         1.8V. SMPS power mode will follow \
                         the system low-power mode",
        SMPSFeedsIntoLDO2V5:
        smps_2v5_feeds_ldo, "VCORE power domains supplied from the LDO. \
                         LDO voltage adjusted by VOS. \
                         LDO power mode will follow the system \
                         low-power mode. SMPS output voltage set to \
                         2.5V. SMPS power mode will follow \
                         the system low-power mode",
    }

    #[cfg(all(
        feature = "revision_v",
        any(feature = "rm0433", feature = "rm0399", feature = "rm0468")
    ))]
    #[must_use]
    pub fn vos0(mut self, _: &SYSCFG) -> Self {
        self.target_vos = VoltageScale::Scale0;
        self
    }
    /// Configure Voltage Scale 1. This is the default configuration
    #[must_use]
    pub fn vos1(mut self) -> Self {
        self.target_vos = VoltageScale::Scale1;
        self
    }
    /// Configure Voltage Scale 2
    #[must_use]
    pub fn vos2(mut self) -> Self {
        self.target_vos = VoltageScale::Scale2;
        self
    }
    /// Configure Voltage Scale 3
    #[must_use]
    pub fn vos3(mut self) -> Self {
        self.target_vos = VoltageScale::Scale3;
        self
    }

    /// Enable the backup domain voltage regulator
    ///
    /// The backup domain voltage regulator maintains the contents of backup SRAM
    /// in Standby and VBAT modes.
    #[must_use]
    pub fn backup_regulator(mut self) -> Self {
        self.backup_regulator = true;
        self
    }

    pub fn freeze(self) -> PowerConfiguration {
        // NB. The lower bytes of CR3 can only be written once after
        // POR, and must be written with a valid combination. Refer to
        // RM0433 Rev 7 6.8.4. This is partially enforced by dropping
        // `self` at the end of this method, but of course we cannot
        // know what happened between the previous POR and here.

        #[cfg(not(feature = "smps"))]
        self.rb.cr3.modify(|_, w| {
            w.scuen().set_bit().ldoen().set_bit().bypass().clear_bit()
        });

        #[cfg(any(feature = "smps"))]
        self.rb.cr3.modify(|_, w| {
            use SupplyConfiguration::*;

            match self.supply_configuration {
                LDOSupply => smps_en!(w).clear_bit().ldoen().set_bit(),
                DirectSMPS => smps_en!(w).set_bit().ldoen().clear_bit(),
                SMPSFeedsIntoLDO1V8 => unsafe {
                    let reg = smps_en!(w).set_bit().ldoen().set_bit();
                    smps_level!(reg).bits(1)
                },
                SMPSFeedsIntoLDO2V5 => unsafe {
                    let reg = smps_en!(w).set_bit().ldoen().set_bit();
                    smps_level!(reg).bits(2)
                },
                Bypass => smps_en!(w)
                    .clear_bit()
                    .ldoen()
                    .clear_bit()
                    .bypass()
                    .set_bit(),
                Default => {
                    // Default configuration. The actual reset value of
                    // CR3 varies between packages (See RM0399 Section
                    // 7.8.4 Footnote 2). Therefore we do not modify
                    // anything here.
                    w
                }
            }
        });
        // Verify supply configuration, panics if these values read
        // from CR3 do not match those written.
        #[cfg(any(feature = "smps"))]
        self.verify_supply_configuration();

        // Validate the supply configuration. If you are stuck here, it is
        // because the voltages on your board do not match those specified
        // in the D3CR.VOS and CR3.SDLEVEL fields.  By default after reset
        // VOS = Scale 3, so check that the voltage on the VCAP pins =
        // 1.0V.
        while self.rb.csr1.read().actvosrdy().bit_is_clear() {}

        // We have now entered Run mode. See RM0433 Rev 7 Section 6.6.1

        // Transition to configured voltage scale. VOS0 cannot be entered
        // directly, instead transition to VOS1 initially and then VOS0 later
        #[allow(unused_mut)]
        let mut vos = match self.target_vos {
            VoltageScale::Scale0 => VoltageScale::Scale1,
            x => x,
        };
        self.voltage_scaling_transition(vos);

        // Enable overdrive for maximum clock
        // Syscfgen required to set enable overdrive
        #[cfg(all(
            feature = "revision_v",
            any(feature = "rm0433", feature = "rm0399")
        ))]
        if matches!(self.target_vos, VoltageScale::Scale0) {
            unsafe {
                &(*RCC::ptr()).apb4enr.modify(|_, w| w.syscfgen().enabled())
            };
            #[cfg(any(feature = "smps"))]
            unsafe {
                &(*SYSCFG::ptr()).pwrcr.modify(|_, w| w.oden().set_bit())
            };
            #[cfg(not(any(feature = "smps")))]
            unsafe {
                &(*SYSCFG::ptr()).pwrcr.modify(|_, w| w.oden().bits(1))
            };
            while d3cr!(self.rb).read().vosrdy().bit_is_clear() {}
            vos = VoltageScale::Scale0;
        }

        // RM0468 chips don't have the overdrive bit
        #[cfg(all(feature = "revision_v", feature = "rm0468"))]
        if matches!(self.target_vos, VoltageScale::Scale0) {
            vos = VoltageScale::Scale0;
            self.voltage_scaling_transition(vos);
            // RM0468 section 6.8.6 says that before being able to use VOS0,
            // D3CR.VOS must equal CSR1.ACTVOS and CSR1.ACTVOSRDY must be set.
            while d3cr!(self.rb).read().vos().bits()
                != self.rb.csr1.read().actvos().bits()
            {}
            while self.rb.csr1.read().actvosrdy().bit_is_clear() {}
        }

        // Disable backup power domain write protection
        self.rb.cr1.modify(|_, w| w.dbp().set_bit());
        while self.rb.cr1.read().dbp().bit_is_clear() {}

        if self.backup_regulator {
            self.rb.cr2.modify(|_, w| w.bren().set_bit());
            while self.rb.cr2.read().brrdy().bit_is_clear() {}
        }

        let backup = unsafe { BackupREC::new_singleton(self.backup_regulator) };

        PowerConfiguration {
            vos,
            backup: Some(backup),
        }
    }
}
