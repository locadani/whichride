package io.github.locadani.whichride.designsystem

import android.content.res.Configuration
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.tooling.preview.Preview

@Preview(name = "Light", widthDp = 390)
@Preview(name = "Dark", widthDp = 390, uiMode = Configuration.UI_MODE_NIGHT_YES)
@Composable
private fun ThemePreview() = WhichrideTheme {
    val space = WhichrideTokens.Space
    Column(
        Modifier.background(MaterialTheme.colorScheme.background).fillMaxWidth().padding(space.md),
        verticalArrangement = Arrangement.spacedBy(space.sm),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(space.sm)) {
            Image(painterResource(R.drawable.whichride_logo), contentDescription = null)
            Text("whichride", style = MaterialTheme.typography.titleLarge, color = MaterialTheme.colorScheme.onBackground)
        }
        Text("Where to?", style = MaterialTheme.typography.headlineLarge, color = MaterialTheme.colorScheme.onBackground)
        Row(
            Modifier.fillMaxWidth()
                .background(WhichrideTheme.colors.hero, RoundedCornerShape(WhichrideTokens.Radius.xl))
                .padding(space.md),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text("MILES · S/M", style = MaterialTheme.typography.titleLarge, color = WhichrideTheme.colors.onHero)
            Text("€8.49", style = WhichrideTheme.numbers.large, color = WhichrideTheme.colors.onHero)
        }
        Text("7.4 km · 22 min", style = WhichrideTheme.numbers.small, color = WhichrideTheme.colors.onSurfaceMuted)
        Button(onClick = {}, Modifier.fillMaxWidth()) { Text("Open MILES") }
        OutlinedButton(onClick = {}, Modifier.fillMaxWidth()) { Text("Add the cars near you") }
        Row(horizontalArrangement = Arrangement.spacedBy(space.sm)) {
            Switch(checked = true, onCheckedChange = {})
            Switch(checked = false, onCheckedChange = {})
        }
    }
}
