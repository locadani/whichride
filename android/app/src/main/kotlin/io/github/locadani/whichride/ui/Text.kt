package io.github.locadani.whichride.ui

import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.withStyle
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.InputError
import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.core.ProviderInfo
import io.github.locadani.whichride.core.Quote
import io.github.locadani.whichride.core.RankedOption
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.format.formatMoney
import io.github.locadani.whichride.prices.PriceSource
import io.github.locadani.whichride.format.percentInput
import io.github.locadani.whichride.personal.AmountOff
import io.github.locadani.whichride.personal.Deal
import io.github.locadani.whichride.personal.Pass
import io.github.locadani.whichride.personal.PercentOff
import io.github.locadani.whichride.personal.RefuelCreditDeal
import io.github.locadani.whichride.personal.isActive
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle
import java.util.Locale

@Composable
fun locale(): Locale = LocalConfiguration.current.locales[0]

@Composable
fun money(cents: Long): String = formatMoney(cents, locale())

/** `title` with its `accent` part in the accent colour ("Where **to?**"). */
@Composable
fun accented(title: String, accent: String): AnnotatedString = buildAnnotatedString {
    val at = title.lastIndexOf(accent)
    if (at < 0) return@buildAnnotatedString append(title)
    append(title.substring(0, at))
    withStyle(SpanStyle(color = WhichrideTheme.colors.accentText)) { append(accent) }
    append(title.substring(at + accent.length))
}

/** "MILES, Bolt Drive or SIXT share". */
@Composable
fun orList(names: List<String>): String = when (names.size) {
    0 -> ""
    1 -> names[0]
    else -> stringResource(R.string.list_or, names.dropLast(1).joinToString(", "), names.last())
}

fun List<ProviderInfo>.nameOf(providerId: String) = firstOrNull { it.id == providerId }?.name ?: providerId

@Composable
fun offerLabel(option: RankedOption): String =
    if (option.choice is Choice.AnyOf) stringResource(R.string.any_car) else option.quote.offerName

/** "cheapest is Kia Picanto · 3 h / 40 km package · confirmed 2 days ago". */
@Composable
fun optionMeta(option: RankedOption, source: PriceSource?): String = buildList {
    if (option.choice is Choice.AnyOf) add(stringResource(R.string.any_car_meta, option.quote.offerName))
    if (option.quote.cardLabel != STANDARD_CARD) add(stringResource(R.string.package_meta, option.quote.cardLabel))
    add(freshness(option.quote, source))
}.joinToString(" · ")

/** §7.5 freshness label. */
@Composable
fun freshness(quote: Quote, source: PriceSource?): String {
    val days = quote.confirmedDaysAgo
    return when {
        source == PriceSource.BUILT_IN -> stringResource(R.string.built_in_price)
        days == null -> stringResource(R.string.not_confirmed)
        days == 0u -> stringResource(R.string.confirmed_today)
        else -> pluralStringResource(R.plurals.confirmed_days_ago, days.toInt(), days.toInt())
    }
}

fun InputError.message() = when (this) {
    InputError.NOT_A_MAPS_LINK -> R.string.error_not_a_maps_link
    InputError.NO_DESTINATION -> R.string.error_no_destination
    InputError.NOT_FOUND -> R.string.error_not_found
    InputError.TOO_LONG -> R.string.error_too_long
}

/** The catalogue's pay-as-you-go card label; any other label names a package. */
private const val STANDARD_CARD = "Standard"

/** "Bolt Drive −20%", "MILES pass", … (canvas Discounts). */
@Composable
fun Deal.title(providers: List<ProviderInfo>): String {
    val provider = providers.nameOf(providerId)
    return when (this) {
        is PercentOff -> stringResource(R.string.deal_percent, provider, percentInput(basisPoints, locale()))
        is AmountOff -> stringResource(R.string.deal_amount, provider, money(cents))
        is Pass -> stringResource(R.string.deal_pass, provider)
        is RefuelCreditDeal -> stringResource(R.string.deal_refuel, provider)
    }
}

/** "until 31 Oct 2026", "once · next trip only", "10% off per km · no unlock fee", "expired". */
@Composable
fun Deal.subtitle(today: Long): String {
    if (this is AmountOff && used) return stringResource(R.string.deal_used)
    if (!isActive(today) && enabled) return stringResource(R.string.deal_expired)
    val loc = locale()
    @Composable fun until(day: Long?): String = day?.let {
        stringResource(R.string.deal_until, LocalDate.ofEpochDay(it).format(DateTimeFormatter.ofLocalizedDate(FormatStyle.MEDIUM).withLocale(loc)))
    } ?: stringResource(R.string.deal_no_end)
    return when (this) {
        is PercentOff -> until(until)
        is AmountOff -> if (once) stringResource(R.string.deal_once) else until(until)
        is Pass -> buildList {
            if (distanceOff > 0) add(stringResource(R.string.deal_pass_km, percentInput(distanceOff, loc)))
            if (timeOff > 0) add(stringResource(R.string.deal_pass_minute, percentInput(timeOff, loc)))
            if (packageOff > 0) add(stringResource(R.string.deal_pass_package, percentInput(packageOff, loc)))
            if (waiveUnlock) add(stringResource(R.string.deal_pass_unlock))
        }.joinToString(" · ")
        is RefuelCreditDeal -> "${money(cents)} · ${stringResource(R.string.deal_refuel_sub)}"
    }
}
