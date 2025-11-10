# Драйвер MCP23S08 (SPI GPIO‑экспандер) — документация (RU)

Этот файл описывает публичный API драйвера `Mcp23s08` (микросхема Microchip **MCP23S08**, 8‑битный GPIO‑экспандер по **SPI**) и вспомогательных типов из `mcp23s08.rs`. Драйвер построен на трэйтах **embedded‑hal 1.0** и совместим с `SpiDevice`.

## Содержание
- [Поддерживаемая периферия](#поддерживаемая-периферия)
- [Ошибки](#ошибки)
- [Перечисления](#перечисления)
- [Основные типы](#основные-типы)
- [Методы `Mcp23s08`](#методы-mcp23s08)
- [Пиновый интерфейс `GpioPin`](#пиновый-интерфейс-gpiopin)
- [Примеры использования](#примеры-использования)
- [Замечания по прерываниям](#замечания-по-прерываниям)
- [Регистры MCP23S08](#регистры-mcp23s08)

---

## Поддерживаемая периферия

- Микросхема: **MCP23S08** (SPI‑вариант семейства MCP23x08).
- Шина: **SPI** через `embedded_hal::spi::SpiDevice`.
- Логический уровень прерываний и режим выхода `INT` настраиваются.

## Ошибки

```rust
pub enum Error<SpiE> {
    Spi(SpiE),
    BadAddress,
}
```
- `Spi(SpiE)` — ошибка, проброшенная из нижележащего SPI‑устройства.
- `BadAddress` — аппаратный адрес `hw_addr` вне допустимого диапазона (0..=3).

## Перечисления

```rust
pub enum Pin { P0, P1, P2, P3, P4, P5, P6, P7 }
```
Логические линии порта GPIO0..GPIO7.

```rust
pub enum Polarity { Normal, Inverted }
```
Полярность входа: обычная или инверсная (маска регистра `IPOL`).

```rust
pub enum InterruptMode { OnChange, CompareToDefault }
```
Режим генерации прерываний: по изменению входа (`INTCON=0`) или сравнением со значением `DEFVAL` (`INTCON=1`).

## Основные типы

```rust
pub struct Mcp23s08<SPI> {
    // скрытые поля: spi, hw_addr, olat, iodir
}
```
Высокоуровневый объект драйвера. Держит ссылку на SPI‑девайс, аппаратный адрес и программные тени регистров `OLAT` и `IODIR` для минимизации чтений.

```rust
pub struct GpioPin<'a, SPI> {
    // dev: &'a mut Mcp23s08<SPI>,
    // pin: Pin,
}
```
Вспомогательная "ручка" на один пин. Реализует трэйты `embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin, ErrorType}` поверх операций `Mcp23s08`.

## Методы `Mcp23s08`

- `new(spi, hw_addr) -> Result<Self, Error<E>>`  
  Создаёт драйвер. Инициализирует `IOCON=0x00`, считывает текущие `IODIR` и `OLAT`. Ошибка `BadAddress` — если `hw_addr > 3`.

- `set_pin_direction(pin, input)` / `set_port_direction(mask)`  
  Настройка направления: `true` → вход, `false` → выход (`IODIR`).

- `set_pin_pullup(pin, enable)` / `set_port_pullups(mask)`  
  Подтяжка к VDD на входах (`GPPU`).

- `set_pin_polarity(pin, pol)`  
  Полярность входного бита (`IPOL`).

- `read_port() -> u8` / `read_pin(pin) -> bool`  
  Чтение состояния порта/пина (`GPIO`).

- `write_port(value)` / `write_pin(pin, high)`  
  Запись логических уровней в порт/пин (`GPIO`). Для согласованности также обновляется программная тень `olat`.

- `write_olat(value)`  
  Прямая запись в `OLAT`.

- Настройка прерываний:  
  - `set_pin_interrupt_enable(pin, enable)` / `set_port_interrupt_enable(mask)` → `GPINTEN`  
  - `set_pin_interrupt_mode(pin, mode)` / `set_port_interrupt_mode(mask)` → `INTCON`/`DEFVAL`  
  - `set_port_default_compare(defval)` → `DEFVAL`  
  - `read_interrupt_flags() -> u8` → `INTF`  
  - `read_interrupt_capture() -> u8` → `INTCAP`  
  - `clear_interrupts() -> u8` — чтение `INTCAP` (сбросит флаги).

- Конфигурация выхода `INT`:  
  - `set_int_open_drain(enable)` — `IOCON.ODR`  
  - `set_int_polarity(active_high)` — `IOCON.INTPOL`

- Удобства:  
  - `pin(pin) -> GpioPin` — получить "ручку" на отдельный пин.  
  - `into_inner(self) -> SPI` — извлечь исходное SPI‑устройство.

## Пиновый интерфейс `GpioPin`

Реализованные трэйты (embedded‑hal 1.0):

- `ErrorType<Error = Error<E>>`
- `InputPin`  
  - `is_high()/is_low()` читают регистр `GPIO` и возвращают уровень выбранного пина.
- `OutputPin`  
  - `set_high()/set_low()` модифицируют `OLAT` и пишут его.  
- `StatefulOutputPin`  
  - `is_set_high()/is_set_low()` используют сохранённую тень `olat` (без чтения с шины).

> Примечание: трэйт `ToggleableOutputPin` не реализован, но может быть добавлен поверх чтения/записи `OLAT`.

## Примеры использования

```rust
use embedded_hal::spi::SpiDevice;
use your_crate::mcp23s08::{Mcp23s08, Pin, Polarity};

fn init<SPI, E>(spi: SPI) -> Mcp23s08<SPI>
where
    SPI: SpiDevice<Error = E>,
{
    // Аппаратный адрес: A2..A0 = 0b001 → hw_addr = 1
    let mut dev = Mcp23s08::new(spi, 1).expect("addr");
    // Направление: P0..P3 — выходы, P4..P7 — входы
    dev.set_port_direction(0b1111_0000).unwrap();
    // Подтяжки на входах
    dev.set_port_pullups(0b1111_0000).unwrap();
    // Полярность входов обычная
    dev.set_pin_polarity(Pin::P4, Polarity::Normal).unwrap();
    dev
}
```

Работа с отдельным пином через `GpioPin`:

```rust
let mut led = dev.pin(Pin::P0);
led.set_high().unwrap();
assert!(led.is_set_high().unwrap()); // проверка по тени OLAT
let level = led.is_high().unwrap();  // реальное чтение GPIO
```

Настройка прерываний "по изменению":

```rust
// Разрешить прерывания на P7..P4
dev.set_port_interrupt_enable(0b1111_0000).unwrap();
// Режим: по изменению уровня (INTCON=0 для соответствующих битов)
dev.set_port_interrupt_mode(0b0000_0000).unwrap();
// INT — активен в высоком уровне и push-pull
dev.set_int_polarity(true).unwrap();
dev.set_int_open_drain(false).unwrap();
```

Сброс флагов прерываний и чтение защёлки:

```rust
if dev.read_interrupt_flags().unwrap() != 0 {
    let latched = dev.clear_interrupts().unwrap(); // читает INTCAP
    // обработка latched...
}
```

## Замечания по прерываниям

- В режиме `InterruptMode::OnChange` (`INTCON=0`) флаг ставится при любом изменении входа.
- В режиме `CompareToDefault` сравнение происходит с `DEFVAL`, используйте `set_port_default_compare`.
- Для сброса флагов необходимо читать `INTCAP`/`GPIO` после источника прерывания.

## Регистры MCP23S08

Драйвер использует следующие регистры (адреса в шестнадцатеричном виде):

```
IODIR(0x00)  IPOL(0x01)  GPINTEN(0x02)  DEFVAL(0x03)
INTCON(0x04) IOCON(0x05) GPPU(0x06)     INTF(0x07)
INTCAP(0x08) GPIO(0x09)  OLAT(0x0A)
```

> Некоторые методы поддерживают «маски портов» (`u8`) для пакетной настройки нескольких линий за один вызов.

---


## Расширенные примеры использования

> Все примеры ниже — «скелеты»; адаптируйте их под вашу HAL/плату. В коде используются трейты **embedded-hal 1.0**.

### 1) Инициализация SPI‑устройства (один чип‑селект)

```rust
use embedded_hal::spi::SpiDevice;
use your_crate::mcp23s08::Mcp23s08;

fn setup<SPI, E>(spi_dev: SPI) -> Mcp23s08<SPI>
where
    SPI: SpiDevice<Error = E>,
{
    // A2..A0 = 0 → аппаратный адрес 0
    let mut mcp = Mcp23s08::new(spi_dev, 0).expect("addr");
    // Сделаем P0..P3 выходами, P4..P7 входами c подтяжками
    mcp.set_port_direction(0b1111_0000).unwrap();
    mcp.set_port_pullups(0b1111_0000).unwrap();
    mcp
}
```

### 2) Создание SpiDevice из шины и CS (пример с embedded-hal-bus)

```rust
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::SpiBus;
use embedded_hal_bus::spi::ExclusiveDevice;
use your_crate::mcp23s08::Mcp23s08;

fn new_with_cs<BUS, CS, E>(bus: BUS, cs: CS) -> Mcp23s08<ExclusiveDevice<BUS, CS>>
where
    BUS: SpiBus<Error = E>,
    CS: OutputPin,
{
    // Критическую секцию выберите под вашу платформу
    let dev = ExclusiveDevice::new(bus, cs, critical_section::with);
    Mcp23s08::new(dev, 0).unwrap()
}
```

### 3) Моргание «софтом» и чтение уровня обратно

```rust
use embedded_hal::delay::DelayNs;
use your_crate::mcp23s08::{Mcp23s08, Pin};

fn blink<SPI, E, D: DelayNs>(mcp: &mut Mcp23s08<SPI>, mut delay: D)
where
    SPI: embedded_hal::spi::SpiDevice<Error = E>,
{
    mcp.set_pin_direction(Pin::P0, false).unwrap(); // выход
    for _ in 0..5 {
        mcp.write_pin(Pin::P0, true).unwrap();
        delay.delay_ms(200);
        // Читаем «как видит порт»
        let state = mcp.read_pin(Pin::P0).unwrap();
        assert!(state);
        mcp.write_pin(Pin::P0, false).unwrap();
        delay.delay_ms(200);
    }
}
```

### 4) Дебаунс кнопок на входах P4..P7 с подтяжками

```rust
use your_crate::mcp23s08::Mcp23s08;

fn setup_buttons<SPI, E>(mcp: &mut Mcp23s08<SPI>)
where
    SPI: embedded_hal::spi::SpiDevice<Error = E>,
{
    // P4..P7 входы с подтяжками, остальное оставим как есть
    let mask_inputs = 0b1111_0000;
    // Направление: 1 = вход
    let mut dir = mcp.read_port().unwrap(); // получим текущее как отправную точку (опционально)
    dir |= mask_inputs;
    mcp.set_port_direction(dir).unwrap();
    mcp.set_port_pullups(mask_inputs).unwrap();
}

fn poll_buttons<SPI, E>(mcp: &mut Mcp23s08<SPI>) -> u8
where
    SPI: embedded_hal::spi::SpiDevice<Error = E>,
{
    // Простейший дебаунс: два чтения с задержкой, совпадающие биты считаем «устойчивыми»
    let a = mcp.read_port().unwrap();
    cortex_m::asm::delay(48_000); // ~1мс @48МГц (замените под вашу платформу)
    let b = mcp.read_port().unwrap();
    !(a ^ b) & 0b1111_0000 // «устойчивые» биты на входах P4..P7 (1 = высокий уровень)
}
```

### 5) Конфигурация аппаратных прерываний: «по изменению» на P6..P7

```rust
use your_crate::mcp23s08::Mcp23s08;

fn enable_change_interrupts<SPI, E>(mcp: &mut Mcp23s08<SPI>)
where
    SPI: embedded_hal::spi::SpiDevice<Error = E>,
{
    // Разрешим прерывания только на P6..P7
    mcp.set_port_interrupt_enable(0b1100_0000).unwrap();
    // Режим: по изменению уровня (INTCON=0 для этих битов)
    mcp.set_port_interrupt_mode(0b0000_0000).unwrap();
    // Линия INT — активный высокий, push‑pull
    mcp.set_int_polarity(true).unwrap();
    mcp.set_int_open_drain(false).unwrap();
}

// Обработчик внешнего прерывания (псевдокод):
#[interrupt]
fn EXTI9_5() {
    // … зачистить флаг прерывания контроллера …
    critical_section::with(|cs| {
        let mcp = unsafe { MCP.borrow(cs).as_mut().unwrap() };
        let flags = mcp.read_interrupt_flags().unwrap();
        if flags != 0 {
            let latched = mcp.clear_interrupts().unwrap(); // чтение INTCAP
            // передать событие в очередь/rtic/embassy и т.д.
            process_gpio_event(flags, latched);
        }
    });
}
```

### 6) Сравнение с DEFVAL: прерывание при отклонении от «нормы»

```rust
use your_crate::mcp23s08::Mcp23s08;

fn enable_compare_interrupts<SPI, E>(mcp: &mut Mcp23s08<SPI>)
where
    SPI: embedded_hal::spi::SpiDevice<Error = E>,
{
    // Нормой считаем все входы «высоко» (подтянуты)
    mcp.set_port_default_compare(0b1111_0000).unwrap();
    // Сравнивать с DEFVAL на входах P4..P7 (INTCON=1 на этих битах)
    mcp.set_port_interrupt_mode(0b1111_0000).unwrap();
    mcp.set_port_interrupt_enable(0b1111_0000).unwrap();
}
```

### 7) Работа с несколькими экспандерами на одной шине

```rust
use embedded_hal::spi::SpiDevice;
use your_crate::mcp23s08::Mcp23s08;

fn two_devices<SPI, E>(spi_a: SPI, spi_b: SPI)
where
    SPI: SpiDevice<Error = E>,
{
    // Вариант A: один SpiDevice, разные аппаратные адреса (A2..A0) — если у вас разведен один CS
    let mut dev0 = Mcp23s08::new(spi_a, 0).unwrap();
    let mut dev1 = Mcp23s08::new(spi_b, 1).unwrap();
    dev0.write_port(0x55).unwrap();
    dev1.write_port(0xAA).unwrap();
}
```

### 8) Масочные операции порта: установка/сброс нескольких выходов

```rust
use your_crate::mcp23s08::Mcp23s08;

fn set_reset_mask<SPI, E>(mcp: &mut Mcp23s08<SPI>, set_mask: u8, reset_mask: u8)
where
    SPI: embedded_hal::spi::SpiDevice<Error = E>,
{
    // Текущая тень OLAT хранится внутри драйвера, поэтому безопасно использовать read‑modify‑write
    let mut olat = mcp.read_port().unwrap(); // можно и держать свою тень
    olat |= set_mask;
    olat &= !reset_mask;
    mcp.write_port(olat).unwrap();
}
```

### 9) Использование `GpioPin` как «ручки» пина (выход/вход)

```rust
use your_crate::mcp23s08::{Mcp23s08, Pin};

fn pin_handle<SPI, E>(mcp: &mut Mcp23s08<SPI>)
where
    SPI: embedded_hal::spi::SpiDevice<Error = E>,
{
    let mut led = mcp.pin(Pin::P0);
    led.set_high().unwrap();
    assert!(led.is_set_high().unwrap()); // проверка по тени (без шины)
    // Превращаем P1 во вход и читаем
    mcp.set_pin_direction(Pin::P1, true).unwrap();
    let btn = mcp.pin(Pin::P1);
    let level = btn.is_high().unwrap();  // реальное чтение GPIO
    let _ = level;
}
```

### 10) Обработка ошибок

```rust
use your_crate::mcp23s08::{Mcp23s08, Error};

fn try_write<SPI, E>(mcp: &mut Mcp23s08<SPI>) -> Result<(), Error<E>>
where
    SPI: embedded_hal::spi::SpiDevice<Error = E>,
{
    mcp.write_port(0x00)?;
    Ok(())
}

// При инициализации:
match Mcp23s08::new(spi_dev, 5) {
    Err(Error::BadAddress) => { /* сообщить о неверном A2..A0 */ }
    Err(Error::Spi(_e)) => { /* обработать ошибку шины */ }
    Ok(mut dev) => { /* … */ }
}
```

### 11) Высокоуровневый пример: светодиод «горит, когда кнопка нажата»

```rust
use embedded_hal::spi::SpiDevice;
use your_crate::mcp23s08::{Mcp23s08, Pin};

fn led_follows_button<SPI, E>(mut dev: Mcp23s08<SPI>)
where
    SPI: SpiDevice<Error = E>,
{
    dev.set_pin_direction(Pin::P0, false).unwrap(); // LED — выход
    dev.set_pin_direction(Pin::P4, true).unwrap();  // BTN — вход
    dev.set_pin_pullup(Pin::P4, true).unwrap();     // подтяжка на кнопке
    loop {
        let pressed = dev.read_pin(Pin::P4).unwrap() == false; // активна «земля»
        dev.write_pin(Pin::P0, pressed).unwrap();
        // добавьте небольшой delay, если нужно
    }
}
```

### 12) Извлечение исходного SPI‑устройства

```rust
let spi_dev = mcp23s08.into_inner(); // полезно для graceful‑shutdown
```
